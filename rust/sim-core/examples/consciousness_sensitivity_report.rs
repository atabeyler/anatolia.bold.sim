use sim_core::consciousness_sensitivity::run_sensitivity_report;

fn main() {
    println!("{:<24} {:<16} {:>14} {:>14} {:>12}", "profile", "term", "baseline_days", "ablated_days", "change_%");
    for row in run_sensitivity_report() {
        let baseline = row.baseline_days.map(|d| d.to_string()).unwrap_or_else(|| "never".to_string());
        let ablated = row.ablated_days.map(|d| d.to_string()).unwrap_or_else(|| "never".to_string());
        let change = row.percent_change.map(|c| format!("{c:+.1}%")).unwrap_or_else(|| "n/a".to_string());
        println!("{:<24} {:<16} {:>14} {:>14} {:>12}", row.profile_name, row.term.label(), baseline, ablated, change);
    }
}
