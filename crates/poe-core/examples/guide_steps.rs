//! Prints the Guide's Steps for one act. Usage: cargo run -p poe-core --example guide_steps -- <act>
use poe_core::guide::{GuideBook, GuideOptions};
fn main() {
    let act: u8 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(8);
    let book = GuideBook::bundled();
    for s in book
        .guide(GuideOptions::default())
        .steps
        .iter()
        .filter(|s| s.act == act)
    {
        println!("{:>6} {:<12} {}", s.id, s.area, s.area_name);
    }
}
