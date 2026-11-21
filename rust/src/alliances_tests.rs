use crate::{alliances::PLAYER, game::Game, save};
const DAY: f64 = 86400.0;

fn grow(g: &mut Game, legs: &[(&str, &str)]) {
    for (a, b) in legs {
        g.lease_aircraft("a320_200").unwrap();
        g.staff_up();
        let plane = g.fleet.last().unwrap().id;
        g.create_route(a, b, plane).unwrap();
        let route = g.routes.last().unwrap().id;
        g.set_ticket_price(route, 120).unwrap();
        g.schedule(route, 1, 240).unwrap();
    }
}
/// Player and both AI airlines hub at EDDF and have three routes.
/// With `rivals`, the second AI also serves EDDM, so the AIs overlap too much to share an alliance.
fn world_with(rivals: bool) -> Game {
    let mut g = Game::default();
    g.found("Alliance audit", "DE", "EDDF").unwrap();
    g.competitors = crate::competition::initial_competitors().unwrap();
    grow(
        &mut g.competitors[0].game,
        &[("EDDF", "LFPG"), ("EDDF", "LEMD")],
    );
    let second: &[(&str, &str)] = if rivals {
        &[("EDDF", "EDDM"), ("EDDF", "LSZH")]
    } else {
        &[("EDDF", "LSZH"), ("EDDF", "LTFJ")]
    };
    grow(&mut g.competitors[1].game, second);
    grow(
        &mut g,
        &[("EDDF", "EGLL"), ("EDDF", "EHAM"), ("EDDF", "EGCC")],
    );
    g.clock.set_running(true);
    g
}
fn world() -> Game {
    world_with(false)
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
fn ai(g: &Game, i: usize) -> String {
    g.competitors[i].game.market_owner_key()
}
fn alliance_of(g: &Game, owner: &str) -> Option<String> {
    g.alliance_of(owner).map(|m| m.alliance.clone())
}
fn days(g: &mut Game, n: f64) {
    assert!(g.advance(n * DAY));
}
fn first_review(g: &mut Game) {
    let due = g.next_alliance_event().unwrap();
    let now = g.clock.elapsed().as_secs();
    assert!(g.advance((due - now) as f64));
    assert_eq!(g.alliances.reviewed_at, due);
}
fn reason(g: &Game, id: &str) -> &'static str {
    g.alliance_check(PLAYER, id).unwrap_err().code
}
fn code(r: Result<(), String>) -> String {
    r.unwrap_err()
}

#[test]
fn ai_airlines_found_and_join_alliances_by_overlap() {
    let mut g = world();
    assert!(g.alliances.is_empty());
    first_review(&mut g);
    let (a, b) = (ai(&g, 0), ai(&g, 1));
    assert_eq!(alliance_of(&g, &a), Some("aurora".into()));
    assert_eq!(alliance_of(&g, &b), Some("aurora".into()));
    assert_eq!(g.alliances.members.len(), 2);
    g.validate_alliances().unwrap();

    let mut g = world_with(true);
    first_review(&mut g);
    let (a, b) = (ai(&g, 0), ai(&g, 1));
    assert_eq!(alliance_of(&g, &a), Some("aurora".into()));
    assert_eq!(alliance_of(&g, &b), Some("meridian".into()));
    assert!(g.alliance_members("zenith").next().is_none());
}

#[test]
fn player_refusals_explain_each_reason() {
    let mut g = world();
    first_review(&mut g);
    assert_eq!(reason(&g, "zenith"), "ALLIANCE_EMPTY");
    g.alliance_check(PLAYER, "aurora").unwrap();

    let mut t = g.clone();
    t.reputation.score = -50 * 10_000;
    let r = t.alliance_check(PLAYER, "aurora").unwrap_err();
    assert_eq!(
        (r.code, r.args),
        ("ALLIANCE_REPUTATION", vec!["-50".into(), "0".into()])
    );
    assert_eq!(code(t.apply_alliance("aurora")), "ALLIANCE_REPUTATION");
    assert!(t.alliances.application.is_none());

    let mut t = g.clone();
    t.stop_schedule(t.routes[0].id).unwrap();
    let r = t.alliance_check(PLAYER, "aurora").unwrap_err();
    assert_eq!(
        (r.code, r.args),
        ("ALLIANCE_SIZE", vec!["2".into(), "3".into()])
    );

    let mut t = g.clone();
    t.airline.as_mut().unwrap().capital_eur = 20_000;
    assert_eq!(reason(&t, "aurora"), "ALLIANCE_CASH");
    t.airline.as_mut().unwrap().capital_eur = -1;
    assert_eq!(reason(&t, "aurora"), "ALLIANCE_RESTRUCTURING");
    let mut t = g.clone();
    t.resolution.deadline = Some(1);
    assert_eq!(reason(&t, "aurora"), "ALLIANCE_RESTRUCTURING");

    let mut t = g.clone();
    for m in t.alliances.members.iter_mut() {
        m.alliance = "aurora".into();
    }
    t.alliances.members.resize(
        crate::alliances::RULES.max_members,
        crate::alliances::Member {
            owner: "company:x".into(),
            alliance: "aurora".into(),
            joined_at: 0,
            leaving_at: None,
        },
    );
    assert_eq!(reason(&t, "aurora"), "ALLIANCE_FULL");

    // The player flies the same airports as an existing member: too much overlap.
    let mut probe = world();
    first_review(&mut probe);
    let member = probe.alliances.members[0].owner.clone();
    let pairs: Vec<(String, String)> = probe
        .competitors
        .iter()
        .find(|c| c.game.market_owner_key() == member)
        .unwrap()
        .game
        .routes
        .iter()
        .map(|r| (r.origin_icao.clone(), r.destination_icao.clone()))
        .collect();
    let legs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let mut copy = Game::default();
    copy.found("Copycat", "DE", "EDDF").unwrap();
    copy.competitors = probe.competitors.clone();
    grow(&mut copy, &legs);
    copy.clock = probe.clock.clone();
    copy.alliances = probe.alliances.clone();
    copy.clock.set_running(true);
    let r = copy.alliance_check(PLAYER, "aurora").unwrap_err();
    assert_eq!(r.code, "ALLIANCE_OVERLAP");
    assert_eq!(r.args[1], "50");
    assert!(r.args[0].parse::<u32>().unwrap() > 50);
}

#[test]
fn apply_join_notice_leave_cooldown_and_attention() {
    let mut g = world();
    first_review(&mut g);
    days(&mut g, 1.0);
    g.apply_alliance("aurora").unwrap();
    assert_eq!(code(g.apply_alliance("aurora")), "ALLIANCE_ACTIVE");
    first_review(&mut g);
    assert_eq!(alliance_of(&g, PLAYER), Some("aurora".into()));
    assert!(g.alliances.application.is_none());
    let joined = g.clock.elapsed().as_secs();
    let accepted: Vec<_> = g
        .attention_snapshot()
        .into_iter()
        .filter(|e| e.kind == "alliance")
        .collect();
    assert_eq!(accepted.len(), 1);
    assert!(accepted[0].text.contains("Aurora Alliance") && accepted[0].text.contains("Aufnahme"));
    let fees_before = g.history.as_ref().unwrap().alliance_fee_through(u64::MAX);
    assert_eq!(fees_before, 0, "nobody pays in the month they join");
    let ai_fee_before = g.competitors[0]
        .game
        .history
        .as_ref()
        .unwrap()
        .alliance_fee_through(u64::MAX);
    assert!(
        ai_fee_before > 0,
        "AI members have been paying since the first review"
    );

    days(&mut g, 10.0);
    g.leave_alliance().unwrap();
    let notice_end = g.alliance_of(PLAYER).unwrap().leaving_at.unwrap();
    assert_eq!(notice_end, g.clock.elapsed().as_secs() + 60 * 86400);
    assert_eq!(code(g.leave_alliance()), "ALLIANCE_LEAVING");
    first_review(&mut g);
    first_review(&mut g);
    assert!(
        g.alliance_of(PLAYER).is_some(),
        "still a member during the notice"
    );
    assert!(g.clock.elapsed().as_secs() < notice_end || g.alliance_of(PLAYER).is_none());
    first_review(&mut g);
    assert!(g.clock.elapsed().as_secs() >= notice_end);
    assert!(g.alliance_of(PLAYER).is_none());
    assert_eq!(joined + 90 * 86400, g.clock.elapsed().as_secs());
    // Fees were paid at the two cycles after joining and none on the exit review.
    let fees = g.history.as_ref().unwrap().alliance_fee_through(u64::MAX);
    assert_eq!(fees, 2 * i128::from(crate::alliances::monthly_fee(&g)));
    assert!(
        g.attention_snapshot()
            .iter()
            .any(|e| e.kind == "alliance" && e.text.contains("Austritt"))
    );
    assert_eq!(reason(&g, "aurora"), "ALLIANCE_COOLDOWN");
    days(&mut g, 61.0);
    assert_ne!(
        g.alliance_check(PLAYER, "aurora").err().map(|r| r.code),
        Some("ALLIANCE_COOLDOWN")
    );
    g.validate_alliances().unwrap();

    // A pending application can be withdrawn without notice.
    let mut g = world();
    first_review(&mut g);
    g.apply_alliance("aurora").unwrap();
    g.leave_alliance().unwrap();
    assert!(g.alliances.application.is_none());
    g.validate_alliances().unwrap();
}

#[test]
fn refused_application_and_removal_are_reported() {
    let mut g = world();
    first_review(&mut g);
    g.apply_alliance("aurora").unwrap();
    g.stop_schedule(g.routes[0].id).unwrap();
    first_review(&mut g);
    assert!(g.alliance_of(PLAYER).is_none());
    let n = g.alliances.notices.last().unwrap();
    assert_eq!(n.kind, "refused");
    assert!(
        n.text.contains("Nur 2 aktive Strecken, mindestens 3"),
        "{}",
        n.text
    );
    assert!(g.attention_snapshot().iter().any(|e| e.kind == "alliance"));

    let mut g = world();
    first_review(&mut g);
    g.apply_alliance("aurora").unwrap();
    first_review(&mut g);
    assert!(g.alliance_of(PLAYER).is_some());
    g.airline.as_mut().unwrap().capital_eur = -5;
    g.history.as_mut().unwrap().opening_balance -= g.cash() + 5 - g.cash();
    first_review(&mut g);
    assert!(g.alliance_of(PLAYER).is_none());
    assert_eq!(g.alliances.notices.last().unwrap().kind, "removed");
}

#[test]
fn rival_alliance_codeshares_end_on_join_and_are_blocked_afterwards() {
    let mut g = world_with(true);
    first_review(&mut g);
    let (a, b) = (ai(&g, 0), ai(&g, 1));
    g.propose_codeshare(&a).unwrap();
    g.propose_codeshare(&b).unwrap();
    g.apply_alliance("aurora").unwrap();
    first_review(&mut g);
    assert_eq!(alliance_of(&g, PLAYER), Some("aurora".into()));
    assert!(g.codeshares.iter().any(|c| c.partner == a));
    assert!(
        g.codeshares.iter().all(|c| c.partner != b),
        "rival alliance codeshare ended"
    );
    assert_eq!(code(g.propose_codeshare(&b)), "CODESHARE_RIVAL_ALLIANCE");
    g.validate_codeshares().unwrap();
}

#[test]
fn members_share_passengers_pay_fees_and_the_ledger_reconciles() {
    let mut g = world();
    first_review(&mut g);
    g.apply_alliance("aurora").unwrap();
    first_review(&mut g);
    days(&mut g, 70.0);
    let h = g.history.as_ref().unwrap();
    let paid = h.alliance_fee_through(u64::MAX);
    assert_eq!(paid, 2 * i128::from(crate::alliances::monthly_fee(&g)));
    assert_eq!(
        i128::from(g.cash()),
        h.balance(g.clock.elapsed().as_secs()).into()
    );
    g.validate_history().unwrap();
    let t = &g.finance.totals;
    assert!(t.connecting_passengers > 0);
    let shared: i64 = t.codeshare.iter().map(|r| r.passengers).sum();
    assert!(
        shared > 0 && shared <= t.connecting_passengers,
        "alliance partners share connections"
    );
    assert!(t.codeshare.iter().map(|r| r.revenue).sum::<i64>() <= t.connecting_revenue);
    for c in &g.competitors {
        let ch = c.game.history.as_ref().unwrap();
        assert!(ch.alliance_fee_through(u64::MAX) > 0);
        assert_eq!(
            i128::from(c.game.cash()),
            ch.balance(c.game.clock.elapsed().as_secs()).into()
        );
    }
    // Pretax profit includes the fee; the ledger row for finances sums it too.
    let none = {
        let mut open = world();
        days(&mut open, 100.0);
        open
    };
    assert!(none.history.as_ref().unwrap().alliance_fees.is_empty());
    assert!(none.finance.totals.codeshare.is_empty());
}

#[test]
fn offers_share_codeshare_partners_and_get_a_demand_bonus_only_for_members() {
    let mut g = world();
    first_review(&mut g);
    let key = g.market_owner_key();
    let offers = g.market_routes();
    assert!(
        offers
            .iter()
            .filter(|o| o.owner == key)
            .all(|o| o.alliance_bonus_bps == 0)
    );
    let ai0 = offers.iter().find(|o| o.owner == ai(&g, 0)).unwrap();
    assert!(
        ai0.alliance_bonus_bps > 0
            && ai0.alliance_bonus_bps <= crate::alliances::RULES.demand_bonus_max_bps
    );
    assert!(ai0.codeshare_partners.contains(&ai(&g, 1)));
    assert!(
        offers
            .iter()
            .filter(|o| o.owner == key)
            .all(|o| o.codeshare_partners.is_empty())
    );
    g.apply_alliance("aurora").unwrap();
    first_review(&mut g);
    let offers = g.market_routes();
    for o in offers.iter().filter(|o| o.owner == key) {
        assert_eq!(o.codeshare_partners.len(), 2);
        assert!(o.alliance_bonus_bps > 0);
    }
}

#[test]
fn insolvent_ai_member_is_dropped_without_breaking_the_rest() {
    let mut g = world();
    first_review(&mut g);
    g.apply_alliance("aurora").unwrap();
    first_review(&mut g);
    let doomed = ai(&g, 0);
    g.competitors[0].game.resolution.insolvent_at = Some(g.clock.elapsed().as_secs());
    g.prune_alliances();
    assert!(g.alliance_of(&doomed).is_none());
    assert_eq!(g.alliances.members.len(), 2);
    days(&mut g, 40.0);
    assert_eq!(g.competitors.len(), 1);
    assert!(g.alliance_of(&ai(&g, 0)).is_some());
    g.validate_alliances().unwrap();
    restart(&g);
}

#[test]
fn ai_member_with_a_shrunken_network_leaves_at_the_next_review() {
    let mut g = world();
    first_review(&mut g);
    let a = ai(&g, 0);
    assert!(g.alliance_of(&a).is_some());
    let due = g.next_alliance_event().unwrap();
    let now = g.clock.elapsed().as_secs();
    assert!(g.advance((due - now - 1) as f64));
    let routes = &mut g.competitors[0].game.routes;
    let keep = routes.len() - 1;
    for r in &mut routes[..keep] {
        r.plan = None;
    }
    g.commercial_changed();
    assert!(g.advance(1.0));
    assert_eq!(g.alliances.reviewed_at, due);
    assert!(g.alliance_of(&a).is_none());
    g.validate_alliances().unwrap();
}

fn scripted(speed: u32, hours_per_step: u32, restart_at: &[u32]) -> Game {
    let mut g = world();
    g.clock.set_speed(speed);
    let steps = 130 * 24 / hours_per_step;
    for step in 0..steps {
        let day = step * hours_per_step / 24;
        let at_day_start = (step * hours_per_step).is_multiple_of(24);
        if at_day_start && day == 31 {
            g.apply_alliance("aurora").unwrap();
        }
        if at_day_start && day == 100 {
            g.leave_alliance().unwrap();
        }
        assert!(g.advance(f64::from(hours_per_step) * 3600.0 / f64::from(speed)));
        if restart_at.contains(&step) {
            g = restart(&g);
            g.clock.set_running(true);
        }
    }
    g.clock.set_speed(1);
    g
}
#[test]
fn alliance_state_is_identical_at_every_speed_step_size_and_restart() {
    let expected = scripted(1, 24, &[]);
    assert!(expected.alliances.members.iter().any(|m| m.owner == PLAYER));
    assert!(
        expected
            .alliances
            .members
            .iter()
            .find(|m| m.owner == PLAYER)
            .unwrap()
            .leaving_at
            .is_some()
    );
    assert!(!expected.history.as_ref().unwrap().alliance_fees.is_empty());
    for (speed, hours, restarts) in [
        (4, 6, vec![]),
        (16, 1, vec![700]),
        (1, 24, vec![40, 41, 90]),
    ] {
        let got = scripted(speed, hours, &restarts);
        assert_eq!(state(&got), state(&expected), "speed={speed} hours={hours}");
    }
}

#[test]
fn old_saves_have_no_alliances_and_ai_airlines_form_them_afterwards() {
    let mut g = world();
    days(&mut g, 10.0);
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("old.json");
    save::save(&p, &g).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    old["version"] = 44.into();
    old["state"].as_object_mut().unwrap().remove("alliances");
    old["state"]["history"]
        .as_object_mut()
        .unwrap()
        .remove("alliance_fees");
    std::fs::write(&p, serde_json::to_vec(&old).unwrap()).unwrap();
    let mut loaded = save::load(&p).unwrap();
    assert!(loaded.alliances.members.is_empty());
    assert_eq!(loaded.alliances.reviewed_at, 0);
    assert!(loaded.history.as_ref().unwrap().alliance_fees.is_empty());
    assert_eq!(loaded.next_alliance_event(), Some(30 * 86400));
    days(&mut loaded, 21.0);
    assert_eq!(loaded.alliances.members.len(), 2);
    let again = restart(&loaded);
    assert_eq!(state(&again), state(&loaded));
}

fn long_run(with_alliances: bool, years: u32) -> (Vec<(String, i64, i128, usize)>, i64) {
    let mut g = world();
    for _ in 0..(years * 12) {
        assert!(g.advance(30.0 * DAY));
        if !with_alliances {
            g.alliances.members.clear();
            g.commercial_changed();
        }
    }
    let rows = g
        .competitors
        .iter()
        .map(|c| {
            let h = c.game.history.as_ref().unwrap();
            (
                c.game.market_owner_key(),
                c.game.cash(),
                h.alliance_fee_through(u64::MAX),
                c.game.routes.len(),
            )
        })
        .collect();
    (rows, g.cash())
}

#[test]
#[ignore = "explicit two-year with/without alliances audit of AI finances"]
fn alliance_ai_finance_audit() {
    for years in [2] {
        let (with, player_with) = long_run(true, years);
        let (without, player_without) = long_run(false, years);
        eprintln!("years={years} player cash with={player_with} without={player_without}");
        for (a, b) in with.iter().zip(&without) {
            eprintln!(
                "{} with: cash {} fees {} routes {} | without: cash {} routes {}",
                a.0, a.1, a.2, a.3, b.1, b.3
            );
        }
    }
}
