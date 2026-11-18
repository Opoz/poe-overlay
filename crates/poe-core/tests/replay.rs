//! Replays a real (scrubbed) Client Log from a Witch campaign run, acts 1-9.

use std::{fs::File, io::BufReader};

use poe_core::log::{LogEvent, LogRecord, replay};

fn fixture() -> Vec<LogRecord> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/witch-acts1-9.log"
    );
    replay(BufReader::new(File::open(path).unwrap())).collect()
}

#[test]
fn every_fixture_line_parses() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/witch-acts1-9.log"
    );
    let lines = std::fs::read_to_string(path).unwrap().lines().count();
    assert_eq!(fixture().len(), lines);
}

#[test]
fn run_starts_with_new_character_in_twilight_strand() {
    let records = fixture();
    let first_area = records.iter().find_map(|r| match &r.event {
        LogEvent::AreaGenerated {
            area_id,
            area_level,
        } => Some((area_id.clone(), *area_level)),
        _ => None,
    });
    assert_eq!(first_area, Some(("1_1_1".into(), 1)));
}

#[test]
fn levels_climb_to_60_and_class_changes_on_ascendancy() {
    let levels: Vec<_> = fixture()
        .into_iter()
        .filter_map(|r| match r.event {
            LogEvent::LevelUp {
                character,
                class,
                level,
            } => Some((character, class, level)),
            _ => None,
        })
        .collect();
    assert!(levels.iter().all(|(c, _, _)| c == "TestExile"));
    assert_eq!(levels.first().unwrap().2, 2);
    assert_eq!(levels.last().unwrap().2, 60);
    assert!(levels.windows(2).all(|w| w[1].2 == w[0].2 + 1));
    assert_eq!(levels.first().unwrap().1, "Witch");
    assert_eq!(levels.last().unwrap().1, "Necromancer");
}

#[test]
fn act_towns_first_reached_in_order() {
    let mut towns: Vec<String> = Vec::new();
    for r in fixture() {
        if let LogEvent::AreaGenerated { area_id, .. } = r.event
            && area_id.ends_with("_town")
            && !towns.contains(&area_id)
        {
            towns.push(area_id);
        }
    }
    assert_eq!(
        towns,
        [
            "1_1_town", "1_2_town", "1_3_town", "1_4_town", "1_5_town", "2_6_town", "2_7_town",
            "2_8_town", "2_9_town"
        ]
    );
}

#[test]
fn twilight_strand_name_is_shared_by_acts_1_and_6() {
    // Area names are not unique; only Area IDs identify an Area.
    let records = fixture();
    let ids_before_strand: Vec<_> = records
        .windows(2)
        .filter_map(|w| match (&w[0].event, &w[1].event) {
            (LogEvent::AreaGenerated { area_id, .. }, LogEvent::AreaEntered { name })
                if name == "The Twilight Strand" =>
            {
                Some(area_id.clone())
            }
            _ => None,
        })
        .collect();
    assert!(ids_before_strand.contains(&"1_1_1".to_string()));
    assert!(ids_before_strand.contains(&"2_6_1".to_string()));
}

#[test]
fn ticks_are_monotonic_within_the_session() {
    let ticks: Vec<u64> = fixture().iter().filter_map(|r| r.tick_ms).collect();
    assert!(ticks.windows(2).all(|w| w[1] >= w[0]));
}
