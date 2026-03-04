//! Seed the "Production Readiness for Live Trading" program (Sprint 111).
//!
//! Idempotent — checks if the program already exists by name before inserting.

use sqlx::PgPool;
use tracing::{info, warn};

use super::error::ProjectError;
use super::repository;

/// Seed the production readiness program with 11 sprints (111-121).
pub async fn seed_production_readiness_program(pool: &PgPool) -> Result<(), ProjectError> {
    const PROGRAM_NAME: &str = "Production Readiness for Live Trading";

    // Idempotency check
    if repository::find_program_by_name(pool, PROGRAM_NAME)
        .await?
        .is_some()
    {
        info!("Project tracking: program '{PROGRAM_NAME}' already seeded, skipping");
        return Ok(());
    }

    info!("Project tracking: seeding '{PROGRAM_NAME}'...");

    let program = repository::insert_program(
        pool,
        PROGRAM_NAME,
        Some("End-to-end production readiness: live market data, broker integration, persistence, security, soak testing, and live cutover"),
        Some("111-121"),
    )
    .await?;

    // Sprint definitions: (number, title, description, gate, tasks)
    let sprint_defs: Vec<SprintDef> = vec![
        SprintDef {
            number: 111,
            title: "Enterprise Project Tracking System",
            desc: "PostgreSQL-backed project/sprint/task tracking with dashboard API",
            gate: None,
            tasks: vec![
                ("SQL migration", "critical"),
                ("Types + error modules", "high"),
                ("Repository (all CRUD)", "high"),
                ("Service (business logic)", "high"),
                ("Seed (production readiness program)", "high"),
                ("Module wiring", "medium"),
                ("API handlers + routes", "high"),
                ("Gate: clippy + tests + sprint closeout", "critical"),
            ],
        },
        SprintDef {
            number: 112,
            title: "Live Market Data Feed — WebSocket Ingestion",
            desc: "Real-time market data via WebSocket from broker APIs",
            gate: Some("A"),
            tasks: vec![
                ("WebSocket client for market data providers", "critical"),
                ("Message deserialization and normalization", "high"),
                ("Reconnection with exponential backoff", "high"),
                ("Channel subscription management", "medium"),
                ("Integration tests with mock WS server", "high"),
            ],
        },
        SprintDef {
            number: 113,
            title: "Market Data Persistence & Staleness Detection",
            desc: "Persist tick data and detect stale feeds for kill switch",
            gate: Some("A"),
            tasks: vec![
                ("TimescaleDB hypertable for tick data", "critical"),
                ("Batch insert pipeline", "high"),
                ("Staleness detector (>48h threshold)", "critical"),
                ("Kill switch integration for stale data", "critical"),
                ("Retention policy and compression", "medium"),
            ],
        },
        SprintDef {
            number: 114,
            title: "Broker Sandbox Integration (IB Paper)",
            desc: "Interactive Brokers paper trading with real API",
            gate: Some("B"),
            tasks: vec![
                ("IB TWS API connection manager", "critical"),
                ("Paper account configuration", "high"),
                ("Order submission via IB API", "critical"),
                ("Position sync from IB", "high"),
                ("Error handling and reconnection", "high"),
            ],
        },
        SprintDef {
            number: 115,
            title: "Order Reconciliation & Fill Handling",
            desc: "Match submitted orders with broker fill reports",
            gate: Some("B"),
            tasks: vec![
                ("Fill event listener", "critical"),
                ("Order state machine (submitted→filled→settled)", "critical"),
                ("Partial fill handling", "high"),
                ("Reconciliation report generation", "medium"),
                ("Alerting on unmatched orders", "high"),
            ],
        },
        SprintDef {
            number: 116,
            title: "Position & Tax Lot DB Persistence",
            desc: "Persist positions and tax lots in PostgreSQL",
            gate: Some("C"),
            tasks: vec![
                ("Positions table with cost basis tracking", "critical"),
                ("Tax lot table (FIFO/LIFO/specific ID)", "critical"),
                ("Position open/close lifecycle", "high"),
                ("Realized P&L calculation", "high"),
                ("Integration with tax module", "medium"),
            ],
        },
        SprintDef {
            number: 117,
            title: "Strategy State Persistence & HRM Weights",
            desc: "Persist strategy parameters and HRM model weights",
            gate: Some("C"),
            tasks: vec![
                ("Strategy state table", "high"),
                ("HRM weights versioning in DB", "critical"),
                ("Checkpoint and restore logic", "high"),
                ("Weight comparison and rollback", "medium"),
                ("Integration tests", "high"),
            ],
        },
        SprintDef {
            number: 118,
            title: "TLS, Secrets Management & Deploy Pipeline",
            desc: "Production security: TLS termination, secrets vault, CI/CD",
            gate: Some("D"),
            tasks: vec![
                ("TLS configuration with Let's Encrypt", "critical"),
                (
                    "Secrets management (env vault or GCP Secret Manager)",
                    "critical",
                ),
                ("Docker production image hardening", "high"),
                ("CI/CD pipeline for staging deploy", "high"),
                ("Health check and readiness probes", "medium"),
            ],
        },
        SprintDef {
            number: 119,
            title: "Kill Switch E2E Verification",
            desc: "End-to-end verification of all kill switch triggers",
            gate: Some("E"),
            tasks: vec![
                ("E2E test: drawdown -10% triggers freeze", "critical"),
                ("E2E test: data staleness >48h blocks trades", "critical"),
                ("E2E test: error rate >5% pauses system", "critical"),
                ("Manual kill switch test", "high"),
                ("Recovery and resume test", "high"),
            ],
        },
        SprintDef {
            number: 120,
            title: "Paper Trading Soak Test (2-4 weeks)",
            desc: "Extended paper trading with real market data to validate stability",
            gate: Some("F"),
            tasks: vec![
                ("Soak test harness setup", "critical"),
                ("Daily P&L and trade log review", "high"),
                ("Stability metrics (uptime, latency, errors)", "critical"),
                ("Memory and resource leak detection", "high"),
                ("Final go/no-go assessment", "critical"),
            ],
        },
        SprintDef {
            number: 121,
            title: "Live Cutover — Minimum Capital",
            desc: "Switch from paper to live trading with minimum capital allocation",
            gate: Some("G"),
            tasks: vec![
                ("Capital allocation plan", "critical"),
                ("Live broker account configuration", "critical"),
                ("Gradual ramp-up schedule", "high"),
                ("Real-time monitoring dashboard", "high"),
                ("Emergency rollback procedure", "critical"),
            ],
        },
    ];

    // Insert sprints
    let mut sprint_ids: Vec<(i32, uuid::Uuid)> = Vec::new();
    for def in &sprint_defs {
        let sprint = repository::insert_sprint(
            pool,
            program.id,
            def.number,
            def.title,
            Some(def.desc),
            def.gate,
            def.tasks.len() as i32,
        )
        .await?;
        sprint_ids.push((def.number, sprint.id));

        // Insert tasks
        for (wp, (title, priority)) in def.tasks.iter().enumerate() {
            repository::insert_task(pool, sprint.id, (wp + 1) as i32, title, None, priority)
                .await?;
        }
    }

    // Insert dependencies
    let deps: &[(i32, &[i32])] = &[
        (112, &[111]),
        (113, &[112]),
        (114, &[112]),
        (115, &[114]),
        (116, &[113]),
        (117, &[116]),
        (118, &[115, 117]),
        (119, &[118]),
        (120, &[119]),
        (121, &[120]),
    ];

    let find_id = |num: i32| -> Option<uuid::Uuid> {
        sprint_ids
            .iter()
            .find(|(n, _)| *n == num)
            .map(|(_, id)| *id)
    };

    for (sprint_num, dep_nums) in deps {
        if let Some(sid) = find_id(*sprint_num) {
            for dep_num in *dep_nums {
                if let Some(did) = find_id(*dep_num) {
                    repository::insert_sprint_dependency(pool, sid, did).await?;
                }
            }
        }
    }

    // Mark Sprint 111 as active (this sprint) and the program as active
    if let Some((_, s111_id)) = sprint_ids.first() {
        let _ = repository::update_sprint_status(pool, *s111_id, "active").await;
    }
    let _ = repository::update_program_status(pool, program.id, "active").await;

    info!(
        "Project tracking: seeded {} sprints with {} total tasks",
        sprint_defs.len(),
        sprint_defs.iter().map(|s| s.tasks.len()).sum::<usize>()
    );

    Ok(())
}

struct SprintDef {
    number: i32,
    title: &'static str,
    desc: &'static str,
    gate: Option<&'static str>,
    tasks: Vec<(&'static str, &'static str)>,
}
