use crate::{game::Game, save};
const DAY: f64 = 86400.0;
fn game() -> Game {
    let mut g = Game::default();
    g.found("Codeshare audit", "DE", "EDDF").unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.competitors[1].game.set_ticket_price(1, 120).unwrap();
    g.competitors[1].game.stop_schedule(1).unwrap();
    g.competitors[1].game.schedule(1, 1, 240).unwrap();
    g.lease_aircraft("a320_200").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 240).unwrap();
    g.clock.set_running(true);
    g
}
fn partner(g: &Game) -> String {
    g.competitors[1].game.market_owner_key()
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("world.json");
    save::save(&p, g).unwrap();
    save::load(&p).unwrap()
}
#[test]
fn codeshare_accept_refuse_end_restructuring_and_frozen_departures() {
    let mut g = game();
    let key = partner(&g);
    let mut refused = g.clone();
    refused.stop_schedule(1).unwrap();
    assert_eq!(
        refused.propose_codeshare(&key).unwrap_err(),
        "CODESHARE_NO_HUB"
    );
    refused = g.clone();
    refused.airline.as_mut().unwrap().capital_eur = -1;
    assert_eq!(
        refused.propose_codeshare(&key).unwrap_err(),
        "CODESHARE_FINANCES"
    );
    refused = g.clone();
    refused.competitors[1].game.resolution.deadline = Some(100);
    assert_eq!(
        refused.propose_codeshare(&key).unwrap_err(),
        "CODESHARE_FINANCES"
    );
    g.propose_codeshare(&key).unwrap();
    assert_eq!(g.propose_codeshare(&key).unwrap_err(), "CODESHARE_EXISTS");
    let mut limited = g.clone();
    limited.codeshares.resize(
        crate::codeshare::RULES.maximum_partners,
        crate::codeshare::Agreement {
            partner: "placeholder".into(),
            started_at: 0,
        },
    );
    let other = limited.competitors[0].game.market_owner_key();
    assert_eq!(
        limited.propose_codeshare(&other).unwrap_err(),
        "CODESHARE_LIMIT"
    );
    assert!(g.advance(60.0));
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone();
    assert!(!frozen.as_ref().unwrap().totals.codeshare.is_empty());
    assert_eq!(state(&g), state(&restart(&g)));
    g.end_codeshare(&key).unwrap();
    let fresh = crate::economy::quote_at_time(
        &g.routes[0],
        "a320_200",
        &g.prioritize_market(&g.market_routes()),
        g.fuel_market.price_at(60),
        60,
    )
    .unwrap();
    assert!(fresh.codeshare.is_empty());
    assert_eq!(frozen, g.routes[0].plan.as_ref().unwrap().airborne);
    let mut restored = restart(&g);
    assert!(g.advance(DAY));
    assert!(restored.advance(DAY));
    assert_eq!(state(&g), state(&restored));
    g.propose_codeshare(&key).unwrap();
    g.competitors[1].game.resolution.deadline = Some(100000);
    g.review_codeshares();
    assert!(g.codeshares.is_empty());
    g.competitors[1].game.resolution.deadline = None;
    g.propose_codeshare(&key).unwrap();
    g.competitors.remove(1);
    g.review_codeshares();
    assert!(g.codeshares.is_empty());
}
#[test]
fn codeshare_steps_speeds_pause_restart_start_and_end_midrun() {
    let mut expected = game();
    let key = partner(&expected);
    assert!(expected.advance(DAY));
    expected.propose_codeshare(&key).unwrap();
    assert!(expected.advance(3.0 * DAY));
    expected.end_codeshare(&key).unwrap();
    assert!(expected.advance(2.0 * DAY));
    for speed in [1, 4, 16] {
        let mut replay = game();
        replay.clock.set_running(false);
        let paused = state(&replay);
        assert!(replay.advance(DAY));
        assert_eq!(paused, state(&replay));
        replay.clock.set_running(true);
        replay.clock.set_speed(speed);
        for hour in 0..144 {
            if hour == 24 {
                replay.propose_codeshare(&key).unwrap();
            }
            if hour == 96 {
                replay.end_codeshare(&key).unwrap();
            }
            assert!(replay.advance(3600.0 / f64::from(speed)));
            if [42, 97].contains(&hour) {
                replay = restart(&replay);
                replay.clock.set_running(false);
                let paused = state(&replay);
                assert!(replay.advance(DAY));
                assert_eq!(state(&replay), paused);
                replay.clock.set_running(true);
            }
        }
        replay.clock.set_speed(1);
        assert_eq!(state(&replay), state(&expected), "speed={speed}");
    }
}
#[test]
fn codeshare_v37_migration_and_receipt_integrity() {
    let mut g = game();
    assert!(g.advance(2.0 * DAY));
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("old.json");
    save::save(&p, &g).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    old["version"] = 37.into();
    old["state"].as_object_mut().unwrap().remove("codeshares");
    std::fs::write(&p, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut loaded = save::load(&p).unwrap();
    assert_eq!(state(&g), state(&loaded));
    assert!(g.advance(DAY));
    assert!(loaded.advance(DAY));
    assert_eq!(state(&g), state(&loaded));
    let key = partner(&g);
    g.propose_codeshare(&key).unwrap();
    assert!(g.advance(3.0 * DAY));
    assert_eq!(state(&g), state(&restart(&g)));
    assert!(!g.finance.totals.codeshare.is_empty());
    assert!(g.finance.totals.codeshare.iter().all(|r| r.revenue > 0));
    let mut invalid = g.clone();
    invalid.codeshares[0].partner = "missing".into();
    assert!(save::save(&p, &invalid).is_err());
}
#[test]
#[ignore = "explicit two/ten-year before/after audit"]
fn codeshare_world_comparison() {
    for active in [false, true] {
        let mut g = Game::default();
        g.found("Mixed EDDF audit", "DE", "EDDF").unwrap();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.set_ticket_price(1, 90).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g.lease_aircraft("a320_200").unwrap();
        g.staff_up();
        g.create_route("EDDF", "LTFM", 2).unwrap();
        g.set_ticket_price(2, 120).unwrap();
        g.schedule(2, 1, 240).unwrap();
        g.clock.set_running(true);
        let key = g.competitors[0].game.market_owner_key();
        if active {
            g.propose_codeshare(&key).unwrap();
        }
        for (years, days) in [(2, 730), (10, 2920)] {
            assert!(g.advance(f64::from(days) * DAY));
            for owner in std::iter::once(&g).chain(g.competitors.iter().map(|c| &*c.game)) {
                println!(
                    "CODESHARE_AUDIT active={active} years={years} owner={} pax={} cash={} revenue={} connecting={} codeshare={:?}",
                    owner.airline.as_ref().unwrap().name,
                    owner.finance.totals.passengers,
                    owner.cash(),
                    owner.finance.totals.revenue,
                    owner.finance.totals.connecting_passengers,
                    owner.finance.totals.codeshare
                );
                assert!(!owner.distressed() && !owner.insolvent());
            }
            restart(&g);
        }
    }
}

#[test]
#[ignore = "write a deterministic real-save fixture for renderer captures"]
fn codeshare_write_ui_fixture() {
    let path = std::env::var("AIRLINE_CODESHARE_FIXTURE").expect("fixture output path");
    let mut g = game();
    g.clock.set_running(false);
    save::save(std::path::Path::new(&path), &g).unwrap();
}

#[test]
fn codeshare_restructuring_event_automatically_ends_both_sides() {
    let mut g = crate::codeshare_standard_tests::standard_eddf_game();
    let key = g.competitors[0].game.market_owner_key();
    g.propose_codeshare(&key).unwrap();
    let grace = crate::financing::RULES.distress_grace_days;
    assert!(g.advance(grace as f64 * DAY + 1.0));
    let c = &mut g.competitors[0].game;
    let change = -1_000_000 - c.cash();
    c.airline.as_mut().unwrap().capital_eur += change;
    c.history.as_mut().unwrap().opening_balance += change;
    c.resolution.distress_since = Some(0);
    assert!(g.advance(1.0));
    assert!(g.competitors[0].game.resolution.deadline.is_some());
    assert!(g.codeshares.is_empty());
    assert!(
        g.market_routes()
            .iter()
            .all(|o| o.codeshare_partners.is_empty())
    );
    assert_eq!(state(&g), state(&restart(&g)));
}

#[test]
fn codeshare_historical_queries_before_first_landing_have_no_receipts() {
    let mut g = game();
    let key = partner(&g);
    g.propose_codeshare(&key).unwrap();
    assert!(g.advance(3.0 * DAY));
    assert!(!g.finance.totals.codeshare.is_empty());
    let mut empty = crate::economy::Totals::default();
    empty.add(&g.finance.totals, 0).unwrap();
    assert_eq!(empty, crate::economy::Totals::default());
    assert_eq!(g.history.as_ref().unwrap().totals(0, 0), empty);
}
