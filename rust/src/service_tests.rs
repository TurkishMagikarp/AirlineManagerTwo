use crate::{demand, economy, game::Game, save, service::Level};

fn game() -> Game {
    let mut g = Game::default();
    g.found("Service", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.buy_aircraft("a320_200").unwrap();
    g.set_cabin_layout(1, 12).unwrap();
    g.staff_up();
    g.create_route("EDDF", "LTFM", 1).unwrap();
    g.set_ticket_price(1, 120).unwrap();
    g.set_business_ticket_price(1, 300).unwrap();
    g.schedule(1, 1, 360).unwrap();
    g
}
fn state(g: &Game) -> serde_json::Value {
    serde_json::to_value(g).unwrap()
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("service.json");
    save::save(&path, g).unwrap();
    save::load(&path).unwrap()
}

#[test]
fn service_per_class_costs_share_shift_and_price_capacity_limits() {
    let mut g = game();
    for level in [Level::Basic, Level::Standard, Level::Premium] {
        g.set_service_level(1, level).unwrap();
        let q = economy::quote_shared(&g.routes[0], "a320_200", &g.market_routes()).unwrap();
        let economy = q.passengers - q.business_passengers;
        assert!(economy > 0 && q.business_passengers > 0);
        let expected = match level {
            Level::Basic => -economy - 4 * q.business_passengers,
            Level::Standard => 0,
            Level::Premium => 3 * economy + 14 * q.business_passengers,
        };
        assert_eq!(q.service, expected);
        assert_eq!(
            q.costs(),
            q.fuel + q.airport + q.crew + q.overhead + expected
        );
        assert_eq!(level.cost(0, 0), 0);
        assert_eq!(level.cost(2, 2), level.rate().business_eur * 2);
        assert!(q.passengers <= q.seats && q.business_passengers <= q.business_seats);
    }
    // High-frequency, large offers leave room for service to affect market share.
    let mut a = g.market_routes()[0].clone();
    a.seats = 1000;
    a.business_seats = 100;
    a.route.service_level = Level::Standard;
    let mut b = a.clone();
    b.route.id = 99;
    let equal = demand::estimate(&a.route, a.seats, &[a.clone(), b.clone()]).unwrap();
    assert_eq!(
        equal,
        demand::estimate(&b.route, b.seats, &[a.clone(), b.clone()]).unwrap()
    );
    a.route.service_level = Level::Premium;
    b.route.service_level = Level::Basic;
    let supply = [a.clone(), b.clone()];
    let premium = demand::estimate(&a.route, a.seats, &supply).unwrap();
    let basic = demand::estimate(&b.route, b.seats, &supply).unwrap();
    assert!(premium.per_departure > equal.per_departure);
    assert!(basic.per_departure < equal.per_departure);
    assert!(premium.per_departure < basic.per_departure * 2);
    assert_eq!(
        premium,
        demand::estimate(&a.route, a.seats, &[b.clone(), a.clone()]).unwrap()
    );
    assert!(
        premium.daily_passengers + basic.daily_passengers
            <= demand::market(&a.route, 0).unwrap().total()
    );
    a.route.ticket_eur = 100_000;
    a.route.business_ticket_eur = 100_000;
    let q = demand::estimate(&a.route, a.seats, &[a.clone(), b]).unwrap();
    assert_eq!(q.per_departure, 0);
}

#[test]
fn service_airborne_snapshot_and_route_groups_are_immutable_and_persistent() {
    let mut g = game();
    g.borrow(8_000_000).unwrap();
    g.buy_aircraft("a320_200").unwrap();
    g.staff_up();
    g.add_route_aircraft(1, 2).unwrap();
    g.set_service_level(1, Level::Premium).unwrap();
    assert!(g.routes.iter().all(|r| r.service_level == Level::Premium));
    g.clock.set_running(true);
    assert!(g.advance(60.0));
    let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
    assert!(frozen.totals.service > 0);
    g.set_service_level(1, Level::Basic).unwrap();
    assert!(g.routes.iter().all(|r| r.service_level == Level::Basic));
    g = restart(&g);
    assert_eq!(
        g.routes[0].plan.as_ref().unwrap().airborne.as_ref(),
        Some(&frozen)
    );
    assert!(g.advance((frozen.landing - 60) as f64));
    assert!(g.finance.recent.contains(&frozen));
    assert!(g.advance(86400.0));
    assert!(g.finance.recent.iter().any(|f| f.totals.service < 0));
    g.validate_history().unwrap();
    assert_eq!(state(&restart(&g)), state(&g));
    let before = state(&g);
    assert!(g.set_service_level(99, Level::Premium).is_err());
    assert!(Level::parse("luxury").is_err());
    assert_eq!(before, state(&g));
}

#[test]
fn service_speeds_steps_restart_and_ai_rules_match() {
    let mut initial = game();
    initial.set_service_level(1, Level::Premium).unwrap();
    initial.competitors = crate::competition::initial_competitors().unwrap();
    let duration = 28 * 86400;
    let mut expected = initial.clone();
    expected.clock.set_running(true);
    assert!(expected.advance(duration as f64));
    assert!(
        expected
            .competitors
            .iter()
            .any(|c| c.game.finance.totals.service > 0)
    );
    expected.validate_competitors().unwrap();
    for speed in [1, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_running(true);
        g.clock.set_speed(speed);
        assert!(g.advance(12345.0 / speed as f64));
        g.set_service_level(1, Level::Basic).unwrap();
        g = restart(&g);
        g.set_service_level(1, Level::Premium).unwrap();
        let mut left = duration - 12345;
        while left > 0 {
            let step = left.min(100_003);
            assert!(g.advance(step as f64 / speed as f64));
            left -= step;
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g), state(&expected));
    }
    let mut ai = game();
    ai.review_service();
    assert_eq!(ai.routes[0].service_level, Level::Premium);
    ai.set_business_ticket_price(1, 121).unwrap();
    ai.review_service();
    assert_eq!(ai.routes[0].service_level, Level::Standard);
}

#[test]
fn service_v17_migration_preserves_neutral_money_history_and_airborne_quotes() {
    let mut g = game();
    g.competitors = crate::competition::initial_competitors().unwrap();
    g.clock.set_running(true);
    assert!(g.advance(360.0 * 60.0 + 60.0));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.json");
    save::save(&path, &g).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("service");
                o.remove("service_level");
                for v in o.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    strip(v);
                }
            }
            _ => {}
        }
    }
    strip(&mut v);
    // Current saves require the fields; older versions explicitly migrate them.
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(save::load(&path).is_err());
    v["version"] = 17.into();
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let loaded = save::load(&path).unwrap();
    g.migrate_tax().unwrap();
    g.marketing.settled_at = g.clock.elapsed().as_secs();
    for c in &mut g.competitors {
        c.game.migrate_tax().unwrap();
        c.game.marketing.settled_at = c.game.clock.elapsed().as_secs();
    }
    assert_eq!(state(&loaded), state(&g));
    let mut advanced = loaded;
    assert!(advanced.advance(86400.0));
    assert!(g.advance(86400.0));
    assert_eq!(state(&advanced), state(&g));
}

#[test]
fn service_rejects_corrupt_signed_costs_and_mismatched_route_groups() {
    let mut g = game();
    g.set_service_level(1, Level::Basic).unwrap();
    g.clock.set_running(true);
    assert!(g.advance(60.0));
    let mut bad = g.clone();
    bad.routes[0]
        .plan
        .as_mut()
        .unwrap()
        .airborne
        .as_mut()
        .unwrap()
        .totals
        .service = i64::MIN;
    assert!(bad.validate_operations().is_err());
    let mut totals = economy::Totals {
        passengers: i64::MAX,
        business_passengers: i64::MAX,
        ..Default::default()
    };
    assert!(crate::service::valid(&totals));
    totals.passengers = 0;
    assert!(!crate::service::valid(&totals));
    g.borrow(8_000_000).unwrap();
    g.buy_aircraft("a320_200").unwrap();
    g.staff_up();
    g.add_route_aircraft(1, 2).unwrap();
    assert!(g.routes.iter().all(|r| r.service_level == Level::Basic));
    g.routes[1].service_level = Level::Premium;
    assert!(g.validate_routes().is_err());
}
