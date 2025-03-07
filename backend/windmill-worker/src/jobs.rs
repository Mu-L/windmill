/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde_json::json;
use sqlx::{Pool, Postgres};
use tracing::instrument;
use uuid::Uuid;
use windmill_common::{
    error::Error,
    flow_status::FlowStatusModule,
    jobs::{get_payload_tag_from_prefixed_path, JobKind, QueuedJob},
    schedule::{schedule_to_user, Schedule},
    users::username_to_permissioned_as,
    METRICS_ENABLED,
};
use windmill_queue::{
    delete_job, push, schedule::get_schedule_opt, QueueTransaction, CLOUD_HOSTED,
};

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<R: rsmq_async::RsmqConnection + Clone + Send>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    logs: String,
    e: serde_json::Value,
    metrics: Option<crate::worker::Metrics>,
    rsmq: Option<R>,
) -> Result<serde_json::Value, Error> {
    if *METRICS_ENABLED {
        metrics.map(|m| m.worker_execution_failed.inc());
    }
    let result = serde_json::json!({ "error": e });
    let _ = add_completed_job(db, &queued_job, false, false, result.clone(), logs, rsmq).await?;
    Ok(result)
}

fn flatten_jobs(modules: Vec<FlowStatusModule>) -> Vec<Uuid> {
    modules
        .into_iter()
        .filter_map(|m| match m {
            FlowStatusModule::Success { job, flow_jobs, .. }
            | FlowStatusModule::Failure { job, flow_jobs, .. } => {
                if let Some(flow_jobs) = flow_jobs {
                    Some(flow_jobs)
                } else {
                    Some(vec![job])
                }
            }
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>()
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job<R: rsmq_async::RsmqConnection + Clone + Send>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: serde_json::Value,
    logs: String,
    rsmq: Option<R>,
) -> Result<Uuid, Error> {
    let is_flow =
        queued_job.job_kind == JobKind::Flow || queued_job.job_kind == JobKind::FlowPreview;
    let duration = if is_flow {
        let jobs = queued_job.parse_flow_status().map(|s| {
            let mut modules = s.modules;
            modules.extend([s.failure_module.module_status]);
            flatten_jobs(modules)
        });
        if let Some(jobs) = jobs {
            sqlx::query_scalar!(
                "SELECT SUM(duration_ms) as duration FROM completed_job WHERE id = ANY($1)",
                jobs.as_slice()
            )
            .fetch_one(db)
            .await
            .ok()
            .flatten()
        } else {
            tracing::warn!("Could not parse flow status");
            None
        }
    } else {
        None
    };

    let mem_peak = sqlx::query_scalar!("SELECT mem_peak FROM queue WHERE id = $1", &queued_job.id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .flatten();
    let mut tx: QueueTransaction<'_, R> = (rsmq, db.begin().await?).into();
    let job_id = queued_job.id.clone();
    let duration = sqlx::query_scalar!(
        "INSERT INTO completed_job AS cj
                   ( workspace_id
                   , id
                   , parent_job
                   , created_by
                   , created_at
                   , started_at
                   , duration_ms
                   , success
                   , script_hash
                   , script_path
                   , args
                   , result
                   , logs
                   , raw_code
                   , raw_lock
                   , canceled
                   , canceled_by
                   , canceled_reason
                   , job_kind
                   , schedule_path
                   , permissioned_as
                   , flow_status
                   , raw_flow
                   , is_flow_step
                   , is_skipped
                   , language
                   , email
                   , visible_to_owner
                   , mem_peak
                   , tag
                )
            VALUES ($1, $2, $3, $4, $5, $6, COALESCE($26, (EXTRACT('epoch' FROM (now())) - EXTRACT('epoch' FROM (COALESCE($6, now()))))*1000), $7, $8, $9,\
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $27, $28, $29, $30)
         ON CONFLICT (id) DO UPDATE SET success = $7, result = $11, logs = concat(cj.logs, $12) RETURNING duration_ms",
        queued_job.workspace_id,
        queued_job.id,
        queued_job.parent_job,
        queued_job.created_by,
        queued_job.created_at,
        queued_job.started_at,
        success,
        queued_job.script_hash.map(|x| x.0),
        queued_job.script_path,
        queued_job.args,
        result,
        logs,
        queued_job.raw_code,
        queued_job.raw_lock,
        queued_job.canceled,
        queued_job.canceled_by,
        queued_job.canceled_reason,
        queued_job.job_kind: JobKind,
        queued_job.schedule_path,
        queued_job.permissioned_as,
        queued_job.flow_status,
        queued_job.raw_flow,
        queued_job.is_flow_step,
        skipped,
        queued_job.language: ScriptLang,
        duration: Option<i64>,
        queued_job.email,
        queued_job.visible_to_owner,
        mem_peak,
        queued_job.tag,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not add completed job {job_id}: {e}")))?;

    tx = delete_job(tx, &queued_job.workspace_id, job_id).await?;
    if !queued_job.is_flow_step
        && queued_job.job_kind != JobKind::Flow
        && queued_job.job_kind != JobKind::FlowPreview
        && queued_job.schedule_path.is_some()
        && queued_job.script_path.is_some()
    {
        tx = handle_maybe_scheduled_job(
            tx,
            db,
            queued_job.schedule_path.as_ref().unwrap(),
            queued_job.script_path.as_ref().unwrap(),
            &queued_job.workspace_id,
            success,
            if success { None } else { Some(result) },
        )
        .await?;
    }
    tx.commit().await?;

    #[cfg(feature = "enterprise")]
    if !is_flow && duration > 1000 {
        let additional_usage = duration / 1000;
        let w_id = &queued_job.workspace_id;
        let premium_workspace = *CLOUD_HOSTED
            && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", w_id)
                .fetch_one(db)
                .await
                .map_err(|e| Error::InternalErr(format!("fetching if {w_id} is premium: {e}")))?;
        let _ = sqlx::query!(
                "INSERT INTO usage (id, is_workspace, month_, usage) 
                VALUES ($1, $2, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 0) 
                ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $3",
                if premium_workspace { w_id } else { &queued_job.email },
                premium_workspace,
                additional_usage)
                .execute(db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")));
    }

    tracing::debug!("Added completed job {}", queued_job.id);
    Ok(queued_job.id)
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_maybe_scheduled_job<'c, R: rsmq_async::RsmqConnection + Clone + Send + 'c>(
    mut tx: QueueTransaction<'c, R>,
    db: &Pool<Postgres>,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
    success: bool,
    result: Option<serde_json::Value>,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    let schedule = get_schedule_opt(tx.transaction_mut(), w_id, schedule_path).await?;

    if schedule.is_none() {
        tracing::error!(
            "Schedule {schedule_path} in {w_id} not found. Impossible to schedule again"
        );
        return Ok(tx);
    }

    let schedule = schedule.unwrap();

    if schedule.enabled && script_path == schedule.script_path {
        if !success {
            if let Some(on_failure_path) = schedule.on_failure.clone() {
                let on_failure_result = handle_on_failure(
                    tx,
                    schedule_path,
                    script_path,
                    w_id,
                    &on_failure_path,
                    result,
                    &schedule.email,
                    &schedule_to_user(&schedule.path),
                    username_to_permissioned_as(&schedule.edited_by),
                )
                .await;

                match on_failure_result {
                    Ok(ntx) => {
                        tx = ntx;
                    }
                    Err(err) => {
                        sqlx::query!(
                        "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                        format!("Could not trigger error handler: {err}"),
                        &schedule.workspace_id,
                        &schedule.path
                    )
                    .execute(db)
                    .await?;
                        tracing::warn!(
                            "Could not trigger error handler for {}: {}",
                            schedule_path,
                            err
                        );
                        return Err(err);
                    }
                }
            }
        }

        let res = windmill_queue::schedule::push_scheduled_job(
            tx,
            Schedule {
                workspace_id: w_id.to_owned(),
                path: schedule.path.clone(),
                edited_by: schedule.edited_by,
                edited_at: schedule.edited_at,
                schedule: schedule.schedule,
                timezone: schedule.timezone,
                enabled: schedule.enabled,
                script_path: schedule.script_path,
                is_flow: schedule.is_flow,
                args: schedule
                    .args
                    .and_then(|e| serde_json::to_value(e).map_or(None, |v| Some(v))),
                extra_perms: serde_json::to_value(schedule.extra_perms).expect("hashmap -> json"),
                email: schedule.email,
                error: None,
                on_failure: schedule.on_failure,
            },
        )
        .await;
        match res {
            Ok(tx) => Ok(tx),
            Err(err) => {
                sqlx::query!(
                    "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                    err.to_string(),
                    &schedule.workspace_id,
                    &schedule.path
                )
                .execute(db)
                .await?;
                tracing::warn!("Could not schedule job for {}: {}", schedule_path, err);
                Err(err)
            }
        }
    } else {
        Ok(tx)
    }
}

async fn handle_on_failure<'c, R: rsmq_async::RsmqConnection + Clone + Send + 'c>(
    mut tx: QueueTransaction<'c, R>,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
    on_failure_path: &str,
    result: Option<serde_json::Value>,
    username: &str,
    email: &str,
    permissioned_as: String,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    let (payload, tag) =
        get_payload_tag_from_prefixed_path(on_failure_path, tx.transaction_mut(), w_id).await?;

    let mut args = result
        .unwrap_or_else(|| json!({}))
        .as_object()
        .unwrap()
        .clone();
    args.insert("schedule_path".to_string(), json!(schedule_path));
    args.insert("path".to_string(), json!(script_path));
    let (uuid, tx) = push(
        tx,
        w_id,
        payload,
        args,
        username,
        email,
        permissioned_as,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        tag,
    )
    .await?;
    tracing::info!(
        "Pushed on_failure job {} for {} to queue",
        uuid,
        schedule_path
    );
    return Ok(tx);
}
