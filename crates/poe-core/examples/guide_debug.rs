//! Replays a Client Log through the Guide and prints where it ends up.
//! Usage: cargo run -p poe-core --example guide_debug -- <Client.txt>
use poe_core::{
    guide::GuideBook,
    log::{LogEvent, replay},
    profile::Profile,
};
fn main() {
    let path = std::env::args().nth(1).expect("path to a Client Log");
    let book = GuideBook::bundled();
    let mut p = Profile::new("replay");
    let mut last_act = 0;
    for r in replay(std::io::BufReader::new(std::fs::File::open(path).unwrap())) {
        if let LogEvent::AreaGenerated { area_id, .. } = r.event {
            book.on_area(&mut p, &area_id);
            let g = book.guide(p.guide_options);
            if let Some(id) = p.guide_step {
                let s = &g.steps[g.index_of(id)];
                if s.act != last_act {
                    println!("act {} reached at area {}", s.act, area_id);
                    last_act = s.act;
                }
            }
        }
    }
    let g = book.guide(p.guide_options);
    let s = &g.steps[g.index_of(p.guide_step.unwrap())];
    println!(
        "final: act {} {} ({}), last area entered {:?}",
        s.act, s.area, s.area_name, p.last_area
    );
}
