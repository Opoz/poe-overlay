//! Prints what the importer reads from a PoB export code file.
//! Usage: cargo run -p poe-core --example pob_dump -- <file with code>
fn main() {
    let path = std::env::args().nth(1).expect("path");
    let code = std::fs::read_to_string(path).unwrap();
    let b = poe_core::pob::import(&code).expect("import");
    println!(
        "{} {:?} lvl {} bandit {:?}",
        b.class, b.ascendancy, b.level, b.bandit
    );
    for (i, g) in b.skill_groups.iter().enumerate() {
        let gems: Vec<_> = g.gems.iter().map(|x| x.name.as_str()).collect();
        println!(
            "G{i:<2} en={} set={:?} label={:?} slot={:?} levels={:?} [{}]",
            g.enabled,
            g.skill_set,
            g.label,
            g.slot,
            g.levels,
            gems.join(", ")
        );
    }
    for (i, s) in b.tree_stages.iter().enumerate() {
        println!(
            "T{i} {:?} nodes={} levels={:?}",
            s.title,
            s.nodes.len(),
            s.levels
        );
    }
}
