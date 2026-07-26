use rand::Rng;
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::types::{Allele, Genome, Locus, Phenotype};

/// Locus table: `(locus_id, chromosome, trait_name, expression_type)`.
const LOCI: &[(&str, &str, &str, &str)] = &[
    ("BDNF_01", "11", "neural_plasticity", "codominant"),
    ("COMT_01", "22", "working_memory", "codominant"),
    ("DTNBP1_01", "6", "fluid_intelligence", "codominant"),
    ("NRG1_01", "8", "cognitive_speed", "codominant"),
    ("DISC1_01", "1", "executive_function", "codominant"),
    ("FOXP2_01", "7", "language_capacity", "codominant"),
    ("CNTNAP2_01", "7", "language_learning", "codominant"),
    ("OXTR_01", "3", "social_bonding", "codominant"),
    ("SLC6A4_01", "17", "serotonin_transport", "codominant"),
    ("DRD4_01", "11", "curiosity", "codominant"),
    ("MAOA_01", "X", "aggression", "x_linked"),
    ("NRXN1_01", "2", "self_awareness", "codominant"),
    ("SHANK3_01", "22", "prefrontal_dev", "codominant"),
    ("RELN_01", "7", "theory_of_mind", "codominant"),
    ("HEIGHT_01", "1", "height", "polygenic"),
    ("HEIGHT_02", "6", "height", "polygenic"),
    ("HEIGHT_03", "12", "height", "polygenic"),
    ("STRENGTH_01", "11", "physical_strength", "codominant"),
    ("METABOLISM_01", "16", "metabolism", "codominant"),
    ("IMMUNE_01", "6", "immune_strength", "codominant"),
    ("IMMUNE_02", "6", "immune_breadth", "codominant"),
    ("TERT_01", "5", "telomere_length", "codominant"),
    ("APOE_01", "19", "longevity", "codominant"),
    ("DRD2_01", "11", "motivation", "codominant"),
    ("AVPR1A_01", "12", "pair_bonding", "codominant"),
    ("ACTN3_01", "11", "muscle_fiber_type", "codominant"),
    ("ADRA2B_01", "2", "memory_consolidation", "codominant"),
    ("CACNA1C_01", "12", "novelty_seeking", "codominant"),
    ("FSHR_01", "2", "fertility", "codominant"),
    ("HERC2_01", "15", "eye_color", "dominant"),
    ("MC1R_01", "16", "hair_color", "codominant"),
    ("SLC24A5_01", "15", "skin_pigmentation", "dominant"),
];

/// Real recombination frequency between two loci depends on their physical
/// distance, not merely whether they share a chromosome number -- two genes
/// tens of megabases apart (e.g. HERC2 and SLC24A5, both annotated to
/// chromosome 15 above but ~20 Mb apart in reality) recombine at close to
/// 50% per meiosis, functionally assorting almost independently, while genes
/// only kilobases-to-low-megabases apart (the MHC/HLA immune complex, which
/// is exactly why IMMUNE_01/IMMUNE_02 are grouped here) really do co-segregate
/// as a block. Treating "same chromosome" as "always fully linked" -- the
/// model's original simplification -- collapsed real per-individual genetic
/// diversity by force-clustering unrelated visible traits (e.g. every
/// blue-eyed descendant would have inherited the exact same skin-pigmentation
/// allele lineage) and understated diversity across whole trait clusters
/// (cognition on chromosome 11, language on chromosome 7, etc). This table is
/// the single, explicit list of the genuinely tightly-linked pairs; every
/// locus not listed here gets its own private linkage group in
/// `linkage_group` below, i.e. assorts independently every gamete -- the
/// correct default for loci whose real physical separation is large even
/// when they happen to share a chromosome annotation.
const LINKED_CLUSTERS: &[(&str, &str)] = &[("IMMUNE_01", "chr6_mhc_cluster"), ("IMMUNE_02", "chr6_mhc_cluster")];

/// The coin-flip key `create_gamete` uses to decide which parental copy a
/// locus inherits. Defaults to the locus's own id (fully independent
/// assortment); only loci in `LINKED_CLUSTERS` share a group and therefore
/// always co-segregate. See `LINKED_CLUSTERS` for the biological rationale.
fn linkage_group(locus_id: &'static str) -> &'static str {
    LINKED_CLUSTERS.iter().find(|(id, _)| *id == locus_id).map(|(_, group)| *group).unwrap_or(locus_id)
}

fn random_allele() -> f64 {
    rand::thread_rng().gen_range(0.1..0.9)
}

fn pick_value(genome: &Genome, locus_id: &str) -> f64 {
    match genome.get(locus_id) {
        Some(locus) => {
            let a1 = locus.allele1.value.unwrap_or_else(random_allele);
            match locus.expression_type.as_str() {
                // Dominant: a single high allele determines the trait, e.g. eye/skin
                // pigmentation -- never the average of a dominant and recessive allele.
                "dominant" => a1.max(locus.allele2.value.unwrap_or(a1)),
                // Hemizygous males express their single (maternal) X-linked allele directly.
                "hemizygous" => a1,
                _ => (a1 + locus.allele2.value.unwrap_or(a1)) / 2.0,
            }
        }
        None => random_allele(),
    }
}

/// `stress_multiplier` scales mutation probability identically across every
/// locus rather than targeting specific genes. This is a deliberate,
/// biologically grounded choice, not an oversight: the mechanism it models
/// -- elevated glucocorticoids driving reactive-oxygen-species accumulation
/// in gametogenic cells -- causes genome-wide oxidative DNA damage, not
/// damage concentrated at particular loci (the same logic behind the
/// well-documented paternal-age germline-mutation effect, which is also
/// genome-wide rather than locus-specific). A locus-targeted stress-mutation
/// model would in fact be *less* realistic here.
fn apply_mutation(value: f64, stress_multiplier: f64) -> f64 {
    let mutation_prob = (2.0 / LOCI.len() as f64) * stress_multiplier;
    if rand::random::<f64>() < mutation_prob {
        let effect = (rand::random::<f64>() - 0.5) * 0.1;
        (value + effect).clamp(0.0, 1.0)
    } else {
        value
    }
}

/// `overrides` uses the simple wire shape callers (the founder-creation API,
/// God Mode) send: `{ "LOCUS_ID": { "a1": 0.8, "a2": 0.8 } }`.
pub fn create_genome(overrides: Option<&Map<String, Value>>) -> Genome {
    let mut genome = Genome::new();
    for (locus_id, chromosome, trait_name, expression_type) in LOCI {
        let existing = overrides.and_then(|m| m.get(*locus_id));
        let a1 = existing.and_then(|l| l.get("a1")).and_then(Value::as_f64).unwrap_or_else(random_allele);
        let a2 = existing.and_then(|l| l.get("a2")).and_then(Value::as_f64).unwrap_or_else(random_allele);
        genome.insert(
            (*locus_id).to_string(),
            Locus {
                locus_id: (*locus_id).to_string(),
                chromosome: Some((*chromosome).to_string()),
                allele1: Allele { value: Some(a1), origin: "paternal".to_string() },
                allele2: Allele { value: Some(a2), origin: "maternal".to_string() },
                expression_type: (*expression_type).to_string(),
                trait_name: (*trait_name).to_string(),
            },
        );
    }
    genome
}

/// Founders have no parents to derive a hemizygous X from the way
/// `combine_gametes` does for every non-founder son -- `create_genome`
/// above builds every locus (X-linked ones included) as if diploid, with
/// two independently-random alleles. Called on a male founder's genome
/// right after creation, this collapses X-linked loci down to the single
/// real allele a man actually has, matching the exact representation
/// (missing `allele2.value`, `expression_type` "hemizygous")
/// `combine_gametes` already produces. Without this, `pick_value`'s `_`
/// catch-all silently averaged his two fabricated "alleles" for MAOA_01
/// instead of expressing his one X directly, and `create_gamete` could
/// pass either fabricated value to a daughter at random -- neither of
/// which a single X chromosome can do.
pub(crate) fn make_x_linked_loci_hemizygous(genome: &mut Genome) {
    for (locus_id, _, _, expression_type) in LOCI {
        if *expression_type != "x_linked" {
            continue;
        }
        if let Some(locus) = genome.get_mut(*locus_id) {
            locus.allele2.value = None;
            locus.expression_type = "hemizygous".to_string();
        }
    }
}

/// Reconstructs `locus_id`/`chromosome`/`trait_name`/`expression_type` on
/// every locus already present in `genome` from the static `LOCI` table,
/// keyed by the genome map's own keys -- the counterpart to `state.rs`'s
/// `serialize_slim_genome`, which drops those same fields before writing to
/// the DB since they never vary per-individual. `expression_type` is the one
/// exception that *does* vary per-individual for X-linked loci (a male's
/// hemizygous single allele vs. a female's two), which is why it isn't just
/// copied from the table verbatim: a missing `allele2.value` is the existing,
/// self-contained signal (set by `combine_gametes` below) that this locus is
/// hemizygous for this individual.
pub(crate) fn hydrate_genome_metadata(genome: &mut Genome) {
    for (locus_id, chromosome, trait_name, expression_type) in LOCI {
        let Some(locus) = genome.get_mut(*locus_id) else { continue };
        locus.locus_id = (*locus_id).to_string();
        locus.chromosome = Some((*chromosome).to_string());
        locus.trait_name = (*trait_name).to_string();
        locus.expression_type =
            if *expression_type == "x_linked" && locus.allele2.value.is_none() { "hemizygous".to_string() } else { (*expression_type).to_string() };
    }
}

pub fn create_gamete(genome: &Genome, stress_multiplier: f64) -> HashMap<String, f64> {
    let mut gamete = HashMap::new();
    // Which parental copy of a locus a gamete carries is decided per
    // *linkage group*, not per chromosome -- see LINKED_CLUSTERS/
    // linkage_group above for why: only genuinely tightly-linked pairs
    // (the MHC/immune cluster) co-segregate as a block, every other locus
    // gets its own independent 50/50 choice even when it shares a
    // chromosome annotation with another locus. Mutation is still rolled
    // independently per locus below regardless of linkage group.
    let mut group_takes_allele2: HashMap<&'static str, bool> = HashMap::new();
    for (locus_id, ..) in LOCI {
        if let Some(locus) = genome.get(*locus_id) {
            let a1 = locus.allele1.value.unwrap_or_else(random_allele);
            // A hemizygous locus (a male's single X, allele2.value == None
            // by convention -- see combine_gametes' own hemizygous branch
            // and make_x_linked_loci_hemizygous below) has only one real
            // allele to pass on. Coin-flipping against a freshly fabricated
            // random a2 -- unwrap_or_else(random_allele) previously did
            // exactly that -- would silently discard his actual X about
            // half the time instead of reliably transmitting it.
            let chosen = if locus.expression_type == "hemizygous" {
                a1
            } else {
                let a2 = locus.allele2.value.unwrap_or_else(random_allele);
                let take_a2 = *group_takes_allele2.entry(linkage_group(locus_id)).or_insert_with(|| rand::random::<f64>() < 0.5);
                if take_a2 { a2 } else { a1 }
            };
            gamete.insert((*locus_id).to_string(), apply_mutation(chosen, stress_multiplier));
        }
    }
    gamete
}

/// `gamete1` is always the mother's gamete and `gamete2` always the
/// father's -- every caller (`create_child`, in both tick.rs's due-birth
/// path and reproduction.rs's own conception path) passes
/// `(mother, father)` in that order. This matters specifically for
/// X-linked loci: a son's single X comes entirely from his mother (his
/// father contributes a Y, never an X), so `a1` -- not `a2` -- is his
/// only possible source. A daughter, and every autosomal locus for
/// either sex, still legitimately gets one copy from each parent.
pub fn combine_gametes(gamete1: &HashMap<String, f64>, gamete2: &HashMap<String, f64>, child_sex: &str) -> Genome {
    let mut genome = Genome::new();
    for (locus_id, chromosome, trait_name, expression_type) in LOCI {
        let a1 = gamete1.get(*locus_id).copied().unwrap_or_else(random_allele);
        let a2 = gamete2.get(*locus_id).copied().unwrap_or_else(random_allele);
        let is_x_linked = *expression_type == "x_linked";
        let is_male = child_sex == "male";
        let (allele1, allele2, expression) = if is_male && is_x_linked {
            (
                Allele { value: Some(a1), origin: "maternal".to_string() },
                Allele { value: None, origin: "hemizygous".to_string() },
                "hemizygous".to_string(),
            )
        } else {
            (
                Allele { value: Some(a1), origin: "maternal".to_string() },
                Allele { value: Some(a2), origin: "paternal".to_string() },
                (*expression_type).to_string(),
            )
        };
        genome.insert(
            (*locus_id).to_string(),
            Locus {
                locus_id: (*locus_id).to_string(),
                chromosome: Some((*chromosome).to_string()),
                allele1,
                allele2,
                expression_type: expression,
                trait_name: (*trait_name).to_string(),
            },
        );
    }
    genome
}

pub fn compute_phenotype(genome: &Genome) -> Phenotype {
    let g = |locus: &str| pick_value(genome, locus);
    let height_base = (g("HEIGHT_01") + g("HEIGHT_02") + g("HEIGHT_03")) / 3.0;
    let language_capacity = (g("FOXP2_01") * 0.75 + g("CNTNAP2_01") * 0.25).min(1.0);
    let fluid_intelligence = (g("BDNF_01") + g("COMT_01") + g("DTNBP1_01") + g("NRG1_01") + g("DISC1_01")) / 5.0;
    let consciousness_potential = (g("NRXN1_01") + g("SHANK3_01") + g("RELN_01") + g("FOXP2_01")) / 4.0;
    let belief_capacity = ((consciousness_potential - 0.1) / 0.9).max(0.0);
    let immune_strength = (g("IMMUNE_01") + g("IMMUNE_02")) / 2.0;
    let max_lifespan = 50.0 + g("TERT_01") * 50.0 + g("APOE_01") * 20.0;

    Phenotype {
        name: None,
        height_factor: height_base,
        physical_strength: (g("STRENGTH_01") * 0.5 + g("HEIGHT_01") * 0.25 + g("METABOLISM_01") * 0.25).min(1.0),
        physical_endurance: g("METABOLISM_01"),
        endurance: (g("ACTN3_01") * 0.5 + g("METABOLISM_01") * 0.3 + g("STRENGTH_01") * 0.2).min(1.0),
        fluid_intelligence,
        working_memory: g("COMT_01"),
        conscientiousness: g("DISC1_01"),
        learning_rate: (g("ADRA2B_01") * 0.4 + g("BDNF_01") * 0.35 + g("COMT_01") * 0.25).min(1.0),
        language_capacity,
        language_learning: g("CNTNAP2_01"),
        social_bonding: g("OXTR_01"),
        social_drive: (g("DRD2_01") * 0.5 + g("OXTR_01") * 0.5).min(1.0),
        oxytocin_sensitivity: g("OXTR_01"),
        empathy: (g("OXTR_01") + g("RELN_01")) / 2.0,
        cooperation: (g("AVPR1A_01") * 0.5 + g("OXTR_01") * 0.35 + (1.0 - g("MAOA_01")) * 0.15).min(1.0),
        altruism: (g("OXTR_01") * 0.7 + (1.0 - g("MAOA_01")) * 0.3).max(0.0),
        parental_care: (g("OXTR_01") * 0.6 + g("AVPR1A_01") * 0.4).min(1.0),
        aggression: g("MAOA_01"),
        dominance: (g("DRD2_01") * 0.5 + g("MAOA_01") * 0.3 + g("DISC1_01") * 0.2).min(1.0),
        curiosity: g("DRD4_01"),
        risk_tolerance: (g("CACNA1C_01") * 0.55 + g("DRD4_01") * 0.35 + (1.0 - g("SLC6A4_01")) * 0.1).min(1.0),
        innovation: ((g("CACNA1C_01") + fluid_intelligence + g("DRD4_01")) / 3.0).min(1.0),
        artistic_sense: (consciousness_potential + g("DRD4_01")) / 2.0,
        serotonin: g("SLC6A4_01"),
        stress_resilience: g("SLC6A4_01"),
        // CACNA1C is one of the most replicated genetic loci for HPA-axis /
        // stress-circuit reactivity in human association studies; low SLC6A4
        // (serotonin transporter) function is the other well-established
        // component (the Caspi et al. 5-HTTLPR x stress-exposure finding),
        // which is why stress_resilience above uses SLC6A4_01 directly and
        // this trait uses its complement -- a low-resilience genotype should
        // also read as a high-reactivity one, not an unrelated coin flip.
        stress_reactivity: (g("CACNA1C_01") * 0.5 + (1.0 - g("SLC6A4_01")) * 0.5).min(1.0),
        health_resilience: (g("SLC6A4_01") * 0.4 + g("STRENGTH_01") * 0.3 + g("TERT_01") * 0.3).min(1.0),
        anxiety: (1.0 - g("SLC6A4_01")).max(0.0),
        independence: (g("DRD4_01") + fluid_intelligence) / 2.0,
        xenophobia: ((1.0 - g("OXTR_01") + g("MAOA_01")) / 2.0).max(0.0),
        metabolism: g("METABOLISM_01"),
        immune_strength,
        max_lifespan: max_lifespan.round(),
        fertility: g("FSHR_01"),
        consciousness_potential,
        belief_capacity,
        religiosity: (belief_capacity * 0.6 + (1.0 - g("SLC6A4_01")) * 0.4).min(1.0),
        self_awareness: (g("NRXN1_01") + g("SHANK3_01")) / 2.0,
        eye_color: if g("HERC2_01") > 0.5 { "brown" } else { "blue" }.to_string(),
        hair_color: if g("MC1R_01") > 0.6 { "dark" } else if g("MC1R_01") > 0.3 { "medium" } else { "light" }.to_string(),
        skin_tone: g("SLC24A5_01"),
        extra: Default::default(),
    }
}

#[derive(Clone, Debug, Default)]
pub struct GenealogyEntry {
    pub parent_1_id: Option<String>,
    pub parent_2_id: Option<String>,
    pub inbreeding_coeff: f64,
}

pub type GenealogyIndex = HashMap<String, GenealogyEntry>;

pub fn compute_inbreeding_coefficient(individual: &crate::state::Individual, genealogy: &GenealogyIndex) -> f64 {
    let Some(p1) = individual.parent_1_id.as_ref() else { return 0.0 };
    let Some(p2) = individual.parent_2_id.as_ref() else { return 0.0 };
    coefficient_of_relationship(p1, p2, genealogy)
}

/// Wright's coefficient of relationship between any two individuals already
/// present in the genealogy index. This is exactly the inbreeding coefficient
/// their child *would* have if they mated -- which is why both
/// `compute_inbreeding_coefficient` (an already-born individual's own F,
/// derived from their two parents' ids) and
/// `reproduction::conception_probability`'s prospective-pair check (two
/// candidate mates, evaluated before any child exists) share this one
/// implementation rather than each individual's own historical
/// `inbreeding_coeff` (which reflects *their parents'* relatedness, not the
/// relatedness of the pair being evaluated now).
pub fn coefficient_of_relationship(id1: &str, id2: &str, genealogy: &GenealogyIndex) -> f64 {
    if !genealogy.contains_key(id1) || !genealogy.contains_key(id2) {
        return 0.0;
    }

    let probs1 = ancestor_probs(id1, genealogy, 10);
    let probs2 = ancestor_probs(id2, genealogy, 10);
    let mut f = 0.0;
    for (anc_id, p1) in probs1 {
        if let Some(p2) = probs2.get(&anc_id) {
            let fa = genealogy.get(&anc_id).map(|e| e.inbreeding_coeff).unwrap_or(0.0);
            f += 0.5 * p1 * p2 * (1.0 + fa);
        }
    }
    f.min(1.0)
}

fn ancestor_probs(start_id: &str, genealogy: &GenealogyIndex, max_depth: usize) -> HashMap<String, f64> {
    let mut probs = HashMap::new();
    let mut stack = vec![(start_id.to_string(), 0usize)];
    let mut visited = std::collections::HashSet::new();
    while let Some((id, depth)) = stack.pop() {
        if depth >= max_depth {
            continue;
        }
        let Some(entry) = genealogy.get(&id) else { continue };
        for pid in [&entry.parent_1_id, &entry.parent_2_id] {
            let Some(pid) = pid else { continue };
            if !genealogy.contains_key(pid) {
                continue;
            }
            *probs.entry(pid.clone()).or_insert(0.0) += 0.5f64.powi((depth + 1) as i32);
            let key = format!("{pid}:{}", depth + 1);
            if visited.insert(key) {
                stack.push((pid.clone(), depth + 1));
            }
        }
    }
    probs
}

/// Population-level genetic-diversity summary, recomputed on every stats/
/// checkpoint refresh (see routes.rs's `derive_stats`). This genome model
/// has no discrete allele identities (each allele is a continuous f64), so
/// the classic textbook heterozygosity/allele-frequency formulas don't
/// apply verbatim -- each metric below is the closest continuous analog:
///
/// - `avg_heterozygosity`: mean |allele1 - allele2| across every
///   non-hemizygous locus of every living individual -- how "mixed" each
///   individual's own two allele copies are, standing in for "fraction of
///   loci with two different alleles". A hemizygous locus (a male's single
///   X, `allele2.value == None`) has no second allele to compare and is
///   skipped rather than counted as zero, which would otherwise drag the
///   average down for every male regardless of his actual diversity.
/// - `allelic_variance`: mean, across loci, of the population variance of
///   each individual's own per-locus allele value (using the single allele
///   for a hemizygous locus, the average of both otherwise) -- how much
///   genetic variation remains in the *gene pool* as a whole, distinct from
///   `avg_heterozygosity`'s per-individual mixedness. This is what
///   drift/bottleneck/inbreeding actually erodes generation over
///   generation, even while individuals themselves can stay heterozygous.
/// - `effective_population_size`: Wright's Ne = 4*Nm*Nf/(Nm+Nf), the
///   standard demographic correction for an unequal sex ratio -- a
///   population skewed heavily toward one sex has a real breeding capacity
///   well below its raw headcount.
/// - `avg_inbreeding_coefficient`: mean of each living individual's own
///   already-tracked `inbreeding_coeff` (Wright's F, see
///   `compute_inbreeding_coefficient`) -- how related the current
///   generation's parents were, on average.
pub fn compute_genetic_diversity(population: &[&crate::state::Individual]) -> Value {
    let living: Vec<&crate::state::Individual> = population.iter().copied().filter(|i| i.alive && !i.is_dead).collect();
    if living.is_empty() {
        return serde_json::json!({
            "avg_heterozygosity": 0.0,
            "allelic_variance": 0.0,
            "effective_population_size": 0.0,
            "avg_inbreeding_coefficient": 0.0,
        });
    }

    let mut heterozygosity_sum = 0.0;
    let mut heterozygosity_n: u32 = 0;
    let mut locus_values: HashMap<&str, Vec<f64>> = HashMap::new();

    for ind in &living {
        for (locus_id, locus) in ind.genome.iter() {
            match (locus.allele1.value, locus.allele2.value) {
                (Some(a1), Some(a2)) => {
                    heterozygosity_sum += (a1 - a2).abs();
                    heterozygosity_n += 1;
                    locus_values.entry(locus_id.as_str()).or_default().push((a1 + a2) / 2.0);
                }
                (Some(a1), None) => {
                    locus_values.entry(locus_id.as_str()).or_default().push(a1);
                }
                _ => {}
            }
        }
    }
    let avg_heterozygosity = if heterozygosity_n > 0 { heterozygosity_sum / heterozygosity_n as f64 } else { 0.0 };

    let mut variance_sum = 0.0;
    let mut variance_n: u32 = 0;
    for values in locus_values.values() {
        if values.len() < 2 {
            continue;
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        variance_sum += values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        variance_n += 1;
    }
    let allelic_variance = if variance_n > 0 { variance_sum / variance_n as f64 } else { 0.0 };

    let male_count = living.iter().filter(|i| i.sex == "male").count() as f64;
    let female_count = living.iter().filter(|i| i.sex == "female").count() as f64;
    let effective_population_size =
        if male_count + female_count > 0.0 { 4.0 * male_count * female_count / (male_count + female_count) } else { 0.0 };

    let avg_inbreeding_coefficient = living.iter().filter_map(|i| i.inbreeding_coeff).sum::<f64>() / living.len() as f64;

    serde_json::json!({
        "avg_heterozygosity": (avg_heterozygosity * 1000.0).round() / 1000.0,
        "allelic_variance": (allelic_variance * 1000.0).round() / 1000.0,
        "effective_population_size": (effective_population_size * 10.0).round() / 10.0,
        "avg_inbreeding_coefficient": (avg_inbreeding_coefficient * 1000.0).round() / 1000.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Individual;

    #[test]
    fn generated_genomes_carry_chromosome_annotations() {
        let genome = create_genome(None);
        for (locus_id, chromosome, ..) in LOCI {
            let locus = genome.get(*locus_id).expect("locus present");
            assert_eq!(locus.chromosome.as_deref(), Some(*chromosome), "{locus_id} chromosome mismatch");
        }
        assert_eq!(genome["FOXP2_01"].chromosome.as_deref(), Some("7"));
        assert_eq!(genome["MAOA_01"].chromosome.as_deref(), Some("X"));
    }

    #[test]
    fn child_genomes_preserve_chromosome_annotations() {
        let g1 = create_gamete(&create_genome(None), 0.0);
        let g2 = create_gamete(&create_genome(None), 0.0);
        let child = combine_gametes(&g1, &g2, "female");
        for (locus_id, chromosome, ..) in LOCI {
            assert_eq!(child[*locus_id].chromosome.as_deref(), Some(*chromosome));
        }
    }

    #[test]
    fn only_the_mhc_immune_cluster_co_segregates_every_other_locus_assorts_independently() {
        // IMMUNE_01/IMMUNE_02 are the one genuinely tightly-linked pair
        // (the real MHC/HLA complex). FOXP2_01/CNTNAP2_01/RELN_01 merely
        // share a chromosome *annotation* (chromosome 7) but are tens of
        // megabases apart in reality, so they must now assort independently
        // -- this is the fix for the old model forcing every same-chromosome
        // trait cluster (language, cognition, appearance) to always co-
        // inherit as a block, which collapsed real genetic diversity.
        let mut overrides = Map::new();
        for locus_id in ["IMMUNE_01", "IMMUNE_02", "FOXP2_01", "CNTNAP2_01", "RELN_01"] {
            overrides.insert(locus_id.to_string(), serde_json::json!({ "a1": 0.05, "a2": 0.95 }));
        }
        let genome = create_genome(Some(&overrides));

        let mut linked_matches = 0u32;
        let mut unlinked_matches = 0u32;
        let trials = 500;
        for _ in 0..trials {
            let gamete = create_gamete(&genome, 0.0);
            let immune1_took_a2 = gamete["IMMUNE_01"] > 0.5;
            let immune2_took_a2 = gamete["IMMUNE_02"] > 0.5;
            let foxp2_took_a2 = gamete["FOXP2_01"] > 0.5;
            let cntnap2_took_a2 = gamete["CNTNAP2_01"] > 0.5;
            let reln_took_a2 = gamete["RELN_01"] > 0.5;
            if immune1_took_a2 == immune2_took_a2 {
                linked_matches += 1;
            }
            if foxp2_took_a2 == cntnap2_took_a2 && foxp2_took_a2 == reln_took_a2 {
                unlinked_matches += 1;
            }
        }
        assert_eq!(linked_matches, trials, "IMMUNE_01/IMMUNE_02 (MHC cluster) must always co-segregate together");
        let unlinked_rate = unlinked_matches as f64 / trials as f64;
        assert!(
            (0.15..0.45).contains(&unlinked_rate),
            "FOXP2/CNTNAP2/RELN no longer share a linkage group and should rarely all three match by chance (~1/8), got {unlinked_rate}"
        );
    }

    #[test]
    fn skin_and_eye_color_no_longer_always_co_segregate_despite_sharing_a_chromosome_annotation() {
        let mut overrides = Map::new();
        overrides.insert("HERC2_01".to_string(), serde_json::json!({ "a1": 0.05, "a2": 0.95 }));
        overrides.insert("SLC24A5_01".to_string(), serde_json::json!({ "a1": 0.05, "a2": 0.95 }));
        let genome = create_genome(Some(&overrides));

        let mut matches = 0u32;
        let trials = 500;
        for _ in 0..trials {
            let gamete = create_gamete(&genome, 0.0);
            if (gamete["HERC2_01"] > 0.5) == (gamete["SLC24A5_01"] > 0.5) {
                matches += 1;
            }
        }
        let rate = matches as f64 / trials as f64;
        assert!((0.35..0.65).contains(&rate), "HERC2_01/SLC24A5_01 should assort ~independently now, got {rate}");
    }

    #[test]
    fn gamete_allele_without_mutation_is_always_one_of_the_two_parental_alleles() {
        let genome = create_genome(None);
        for _ in 0..200 {
            let gamete = create_gamete(&genome, 0.0);
            for (locus_id, ..) in LOCI {
                let a1 = genome[*locus_id].allele1.value.unwrap();
                let a2 = genome[*locus_id].allele2.value.unwrap();
                let chosen = gamete[*locus_id];
                assert!((chosen - a1).abs() < 1e-9 || (chosen - a2).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn child_genome_is_built_only_from_the_two_gametes_no_third_source() {
        let g1 = create_gamete(&create_genome(None), 0.0);
        let g2 = create_gamete(&create_genome(None), 0.0);
        let child = combine_gametes(&g1, &g2, "female");
        for (locus_id, ..) in LOCI {
            assert_eq!(child[*locus_id].allele1.value, Some(g1[*locus_id]));
            assert_eq!(child[*locus_id].allele2.value, Some(g2[*locus_id]));
        }
    }

    // Regression test for a real inheritance-direction bug: combine_gametes
    // used to source a son's single X-linked value from `gamete2` and label
    // it "maternal" -- but every caller (create_child, both in tick.rs and
    // reproduction.rs) passes (mother, father) in that order, so gamete2 is
    // always the father. A son's X can only ever come from his mother
    // (fathers contribute a Y, never an X), so this was silently sourcing
    // his MAOA_01-driven aggression phenotype from the wrong parent.
    #[test]
    fn x_linked_locus_gives_males_a_single_hemizygous_maternal_allele() {
        let mother_gamete = create_gamete(&create_genome(None), 0.0);
        let father_gamete = create_gamete(&create_genome(None), 0.0);
        let son = combine_gametes(&mother_gamete, &father_gamete, "male");
        assert_eq!(son["MAOA_01"].allele1.value, Some(mother_gamete["MAOA_01"]), "a son's only X must come from his mother, not his father");
        assert_eq!(son["MAOA_01"].allele1.origin, "maternal");
        assert_eq!(son["MAOA_01"].expression_type, "hemizygous");
        assert!(son["MAOA_01"].allele2.value.is_none());
        assert_eq!(son["MAOA_01"].chromosome.as_deref(), Some("X"));

        let daughter = combine_gametes(&mother_gamete, &father_gamete, "female");
        assert_eq!(daughter["MAOA_01"].allele1.value, Some(mother_gamete["MAOA_01"]));
        assert_eq!(daughter["MAOA_01"].allele1.origin, "maternal");
        assert_eq!(daughter["MAOA_01"].allele2.value, Some(father_gamete["MAOA_01"]));
        assert_eq!(daughter["MAOA_01"].allele2.origin, "paternal");
    }

    // Regression test for the founder-genetics bug: create_genome built
    // every locus (X-linked MAOA_01 included) as diploid regardless of
    // sex, so a male founder's single X was modeled as two independently-
    // random "alleles" -- pick_value's `_` catch-all then averaged them
    // instead of expressing one X directly. make_x_linked_loci_hemizygous
    // (called from create_founder for sex == "male") must collapse this
    // down to the same representation combine_gametes already produces
    // for a non-founder son.
    #[test]
    fn make_x_linked_loci_hemizygous_collapses_a_male_founders_two_alleles_to_one() {
        let mut genome = create_genome(None);
        let original_a1 = genome["MAOA_01"].allele1.value;
        make_x_linked_loci_hemizygous(&mut genome);
        assert_eq!(genome["MAOA_01"].expression_type, "hemizygous");
        assert!(genome["MAOA_01"].allele2.value.is_none());
        assert_eq!(genome["MAOA_01"].allele1.value, original_a1, "allele1 must be left untouched, not re-rolled");
        // Autosomal loci are unaffected -- only X-linked ones collapse.
        assert_eq!(genome["BDNF_01"].expression_type, "codominant");
        assert!(genome["BDNF_01"].allele2.value.is_some());
    }

    #[test]
    fn a_hemizygous_founders_aggression_reflects_his_single_allele_not_an_average() {
        let overrides = serde_json::json!({ "MAOA_01": { "a1": 0.9, "a2": 0.1 } });
        let mut genome = create_genome(Some(overrides.as_object().unwrap()));
        make_x_linked_loci_hemizygous(&mut genome);
        let phenotype = compute_phenotype(&genome);
        // Before the fix this would be (0.9 + 0.1) / 2.0 == 0.5 -- the
        // average of a fabricated second "allele" a man never has.
        assert_eq!(phenotype.aggression, 0.9);
    }

    // Regression test for the companion create_gamete bug: a hemizygous
    // locus (allele2.value == None) used to still get a fabricated
    // random a2 via unwrap_or_else, then coin-flip against it -- silently
    // discarding the real allele about half the time. With mutation
    // disabled (stress_multiplier 0.0) the gamete's value for a
    // hemizygous locus must now deterministically equal allele1 on every
    // single call, not just "one of two values" the way
    // gamete_allele_without_mutation_is_always_one_of_the_two_parental_alleles
    // checks for ordinary diploid loci above.
    #[test]
    fn gamete_from_a_hemizygous_locus_always_carries_the_real_allele_not_fabricated_noise() {
        let mut genome = create_genome(None);
        make_x_linked_loci_hemizygous(&mut genome);
        let a1 = genome["MAOA_01"].allele1.value.unwrap();
        for _ in 0..200 {
            let gamete = create_gamete(&genome, 0.0);
            assert_eq!(gamete["MAOA_01"], a1);
        }
    }

    #[test]
    fn mutation_probability_zero_never_perturbs_the_value() {
        for _ in 0..500 {
            assert_eq!(apply_mutation(0.42, 0.0), 0.42);
        }
    }

    #[test]
    fn compute_phenotype_is_a_pure_function_of_the_genome() {
        let genome = create_genome(None);
        let p1 = compute_phenotype(&genome);
        let p2 = compute_phenotype(&genome);
        assert_eq!(p1, p2, "identical genome must always yield identical phenotype");
    }

    fn make_individual(id: &str, parent_1: Option<&str>, parent_2: Option<&str>, inbreeding: f64) -> Individual {
        Individual {
            id: id.to_string(),
            parent_1_id: parent_1.map(str::to_string),
            parent_2_id: parent_2.map(str::to_string),
            inbreeding_coeff: Some(inbreeding),
            ..Default::default()
        }
    }

    fn genealogy_entry(parent_1: Option<&str>, parent_2: Option<&str>, inbreeding: f64) -> GenealogyEntry {
        GenealogyEntry {
            parent_1_id: parent_1.map(str::to_string),
            parent_2_id: parent_2.map(str::to_string),
            inbreeding_coeff: inbreeding,
        }
    }

    #[test]
    fn full_sibling_mating_yields_expected_inbreeding_coefficient() {
        let mut genealogy = GenealogyIndex::new();
        genealogy.insert("dad".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("mom".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("sib1".to_string(), genealogy_entry(Some("dad"), Some("mom"), 0.0));
        genealogy.insert("sib2".to_string(), genealogy_entry(Some("dad"), Some("mom"), 0.0));
        let child = make_individual("child", Some("sib1"), Some("sib2"), 0.0);
        let f = compute_inbreeding_coefficient(&child, &genealogy);
        assert!((f - 0.25).abs() < 1e-6, "expected F ~= 0.25 for full-sibling mating, got {f}");
    }

    #[test]
    fn unrelated_parents_yield_zero_inbreeding_coefficient() {
        let mut genealogy = GenealogyIndex::new();
        genealogy.insert("a".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("b".to_string(), genealogy_entry(None, None, 0.0));
        let child = make_individual("child", Some("a"), Some("b"), 0.0);
        assert_eq!(compute_inbreeding_coefficient(&child, &genealogy), 0.0);
    }

    #[test]
    fn an_individual_with_no_recorded_parents_has_zero_inbreeding_coefficient() {
        let genealogy = GenealogyIndex::new();
        let solo = make_individual("x", None, None, 0.0);
        assert_eq!(compute_inbreeding_coefficient(&solo, &genealogy), 0.0);
    }

    #[test]
    fn half_sibling_mating_yields_expected_inbreeding_coefficient() {
        let mut genealogy = GenealogyIndex::new();
        genealogy.insert("gp1".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("gp2".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("gp3".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("parent1".to_string(), genealogy_entry(Some("gp1"), Some("gp2"), 0.0));
        genealogy.insert("parent2".to_string(), genealogy_entry(Some("gp1"), Some("gp3"), 0.0));
        let child = make_individual("child", Some("parent1"), Some("parent2"), 0.0);
        let f = compute_inbreeding_coefficient(&child, &genealogy);
        assert!((f - 0.125).abs() < 1e-6, "expected F ~= 0.125 for half-sibling mating, got {f}");
    }

    #[test]
    fn first_cousin_mating_yields_expected_inbreeding_coefficient() {
        let mut genealogy = GenealogyIndex::new();
        genealogy.insert("gp1".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("gp2".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("gp3".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("gp4".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("sib1".to_string(), genealogy_entry(Some("gp1"), Some("gp2"), 0.0));
        genealogy.insert("sib2".to_string(), genealogy_entry(Some("gp1"), Some("gp2"), 0.0));
        genealogy.insert("cousin1".to_string(), genealogy_entry(Some("sib1"), Some("gp3"), 0.0));
        genealogy.insert("cousin2".to_string(), genealogy_entry(Some("sib2"), Some("gp4"), 0.0));
        let child = make_individual("child", Some("cousin1"), Some("cousin2"), 0.0);
        let f = compute_inbreeding_coefficient(&child, &genealogy);
        assert!((f - 0.0625).abs() < 1e-6, "expected F ~= 0.0625 for first-cousin mating, got {f}");
    }

    #[test]
    fn an_ancestor_missing_from_the_genealogy_index_is_silently_excluded() {
        let mut genealogy = GenealogyIndex::new();
        genealogy.insert("dad".to_string(), genealogy_entry(None, None, 0.0));
        genealogy.insert("sib1".to_string(), genealogy_entry(Some("dad"), Some("mom"), 0.0));
        genealogy.insert("sib2".to_string(), genealogy_entry(Some("dad"), Some("mom"), 0.0));
        let child = make_individual("child", Some("sib1"), Some("sib2"), 0.0);
        let f = compute_inbreeding_coefficient(&child, &genealogy);
        assert!((f - 0.125).abs() < 1e-6, "expected F ~= 0.125 when one ancestor is missing, got {f}");
    }

    #[test]
    fn create_genome_contains_all_loci_within_zero_one() {
        let genome = create_genome(None);
        for (locus_id, ..) in LOCI {
            let locus = genome.get(*locus_id).expect("locus present");
            let v = locus.allele1.value.expect("allele1 set");
            assert!((0.0..=1.0).contains(&v));
        }
        assert_eq!(genome.len(), LOCI.len());
    }

    #[test]
    fn create_genome_applies_overrides() {
        let mut overrides = Map::new();
        overrides.insert("FOXP2_01".to_string(), serde_json::json!({ "a1": 0.9, "a2": 0.85 }));
        let genome = create_genome(Some(&overrides));
        assert_eq!(genome["FOXP2_01"].allele1.value, Some(0.9));
        assert_eq!(genome["FOXP2_01"].allele2.value, Some(0.85));
        assert_eq!(genome["FOXP2_01"].chromosome.as_deref(), Some("7"));
    }

    #[test]
    fn gamete_has_one_value_per_locus_within_zero_one() {
        let genome = create_genome(None);
        let gamete = create_gamete(&genome, 1.0);
        for (locus_id, ..) in LOCI {
            let v = gamete[*locus_id];
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn mutation_rate_averages_about_two_per_gamete() {
        let mut overrides = Map::new();
        for (locus_id, ..) in LOCI {
            overrides.insert((*locus_id).to_string(), serde_json::json!({ "a1": 0.5, "a2": 0.5 }));
        }
        let trials = 10_000;
        let mut total_mutations = 0u64;
        for _ in 0..trials {
            let genome = create_genome(Some(&overrides));
            let gamete = create_gamete(&genome, 1.0);
            for (locus_id, ..) in LOCI {
                if (gamete[*locus_id] - 0.5).abs() > 1e-9 {
                    total_mutations += 1;
                }
            }
        }
        let avg_per_gamete = total_mutations as f64 / trials as f64;
        assert!(avg_per_gamete > 0.8 && avg_per_gamete < 4.0, "expected ~2 mutations/gamete, got {avg_per_gamete}");
    }

    #[test]
    fn stress_multiplier_increases_mutation_probability() {
        let mut overrides = Map::new();
        for (locus_id, ..) in LOCI {
            overrides.insert((*locus_id).to_string(), serde_json::json!({ "a1": 0.5, "a2": 0.5 }));
        }
        let trials = 5000;
        let mut normal_muts = 0u64;
        let mut stress_muts = 0u64;
        for _ in 0..trials {
            let g_normal = create_gamete(&create_genome(Some(&overrides)), 1.0);
            let g_stress = create_gamete(&create_genome(Some(&overrides)), 3.0);
            for (locus_id, ..) in LOCI {
                if (g_normal[*locus_id] - 0.5).abs() > 1e-9 {
                    normal_muts += 1;
                }
                if (g_stress[*locus_id] - 0.5).abs() > 1e-9 {
                    stress_muts += 1;
                }
            }
        }
        assert!(stress_muts > normal_muts);
    }

    #[test]
    fn combine_gametes_produces_a_complete_child_genome() {
        let g1 = create_gamete(&create_genome(None), 1.0);
        let g2 = create_gamete(&create_genome(None), 1.0);
        let child = combine_gametes(&g1, &g2, "female");
        assert_eq!(child.len(), LOCI.len());
    }

    #[test]
    fn compute_phenotype_returns_all_expected_traits_within_range() {
        let phenotype = compute_phenotype(&create_genome(None));
        assert!(phenotype.fluid_intelligence >= 0.0);
        assert!((0.0..=1.0).contains(&phenotype.consciousness_potential));
        assert!(phenotype.max_lifespan > 50.0);
        assert!(phenotype.language_capacity >= 0.0);
    }

    #[test]
    fn high_foxp2_alleles_yield_high_language_capacity() {
        let mut overrides = Map::new();
        overrides.insert("FOXP2_01".to_string(), serde_json::json!({ "a1": 0.99, "a2": 0.99 }));
        let phenotype = compute_phenotype(&create_genome(Some(&overrides)));
        assert!(phenotype.language_capacity > 0.7);
    }

    #[test]
    fn belief_capacity_is_derived_from_consciousness_potential() {
        let mut overrides = Map::new();
        for locus in ["NRXN1_01", "SHANK3_01", "RELN_01", "FOXP2_01"] {
            overrides.insert(locus.to_string(), serde_json::json!({ "a1": 0.99, "a2": 0.99 }));
        }
        let phenotype = compute_phenotype(&create_genome(Some(&overrides)));
        assert!(phenotype.belief_capacity > 0.0);
    }

    #[test]
    fn dominant_expression_takes_the_higher_allele_not_the_average() {
        let mut overrides = Map::new();
        overrides.insert("HERC2_01".to_string(), serde_json::json!({ "a1": 0.95, "a2": 0.05 }));
        let phenotype = compute_phenotype(&create_genome(Some(&overrides)));
        assert_eq!(phenotype.eye_color, "brown");
    }

    // ── compute_genetic_diversity() ─────────────────────────────────────

    fn locus(a1: f64, a2: Option<f64>) -> Locus {
        Locus {
            locus_id: "TEST_01".to_string(),
            chromosome: Some("1".to_string()),
            allele1: Allele { value: Some(a1), origin: "paternal".to_string() },
            allele2: Allele { value: a2, origin: "maternal".to_string() },
            expression_type: if a2.is_some() { "codominant".to_string() } else { "hemizygous".to_string() },
            trait_name: "test_trait".to_string(),
        }
    }

    fn individual_with(sex: &str, alive: bool, is_dead: bool, inbreeding: Option<f64>, genome: Genome) -> Individual {
        Individual { sex: sex.to_string(), alive, is_dead, inbreeding_coeff: inbreeding, genome, ..Default::default() }
    }

    #[test]
    fn no_living_individuals_yields_all_zeroed_stats() {
        let dead = individual_with("female", false, true, Some(0.1), HashMap::new());
        let stats = compute_genetic_diversity(&[&dead]);
        assert_eq!(stats["avg_heterozygosity"], 0.0);
        assert_eq!(stats["allelic_variance"], 0.0);
        assert_eq!(stats["effective_population_size"], 0.0);
        assert_eq!(stats["avg_inbreeding_coefficient"], 0.0);
    }

    #[test]
    fn identical_homozygous_genomes_yield_zero_heterozygosity() {
        let mut g1 = HashMap::new();
        g1.insert("TEST_01".to_string(), locus(0.5, Some(0.5)));
        let a = individual_with("male", true, false, Some(0.0), g1.clone());
        let b = individual_with("female", true, false, Some(0.0), g1);
        let stats = compute_genetic_diversity(&[&a, &b]);
        assert_eq!(stats["avg_heterozygosity"], 0.0);
    }

    #[test]
    fn maximally_different_alleles_yield_heterozygosity_of_one() {
        let mut g = HashMap::new();
        g.insert("TEST_01".to_string(), locus(0.0, Some(1.0)));
        let ind = individual_with("male", true, false, Some(0.0), g);
        let stats = compute_genetic_diversity(&[&ind]);
        assert_eq!(stats["avg_heterozygosity"], 1.0);
    }

    // The whole point of tracking both metrics separately: two individuals
    // can each be perfectly homozygous (zero personal heterozygosity) while
    // the *population* still spans the full range of allele values -- a
    // single number can't distinguish "everyone is a uniform clone" from
    // "the gene pool is split between two extremes".
    #[test]
    fn allelic_variance_is_nonzero_even_when_every_individual_is_homozygous() {
        let mut g_low = HashMap::new();
        g_low.insert("TEST_01".to_string(), locus(0.0, Some(0.0)));
        let mut g_high = HashMap::new();
        g_high.insert("TEST_01".to_string(), locus(1.0, Some(1.0)));
        let a = individual_with("male", true, false, Some(0.0), g_low);
        let b = individual_with("female", true, false, Some(0.0), g_high);
        let stats = compute_genetic_diversity(&[&a, &b]);
        assert_eq!(stats["avg_heterozygosity"], 0.0, "both individuals are fully homozygous");
        assert_eq!(stats["allelic_variance"], 0.25, "the gene pool is still split between 0.0 and 1.0");
    }

    #[test]
    fn hemizygous_loci_are_excluded_from_heterozygosity_but_still_feed_allelic_variance() {
        let mut g1 = HashMap::new();
        g1.insert("TEST_01".to_string(), locus(0.3, None));
        let mut g2 = HashMap::new();
        g2.insert("TEST_01".to_string(), locus(0.7, None));
        let a = individual_with("male", true, false, Some(0.0), g1);
        let b = individual_with("male", true, false, Some(0.0), g2);
        let stats = compute_genetic_diversity(&[&a, &b]);
        assert_eq!(stats["avg_heterozygosity"], 0.0, "no locus here has a second allele to compare");
        assert_eq!(stats["allelic_variance"], 0.04, "variance of {{0.3, 0.7}} around mean 0.5");
    }

    #[test]
    fn dead_and_non_alive_individuals_never_influence_the_result() {
        let mut g_alive = HashMap::new();
        g_alive.insert("TEST_01".to_string(), locus(0.5, Some(0.5)));
        let mut g_dead = HashMap::new();
        g_dead.insert("TEST_01".to_string(), locus(0.0, Some(1.0)));
        let alive = individual_with("male", true, false, Some(0.0), g_alive);
        let dead = individual_with("female", false, true, Some(0.9), g_dead);
        let stats = compute_genetic_diversity(&[&alive, &dead]);
        assert_eq!(stats["avg_heterozygosity"], 0.0, "only the living individual's homozygous locus should count");
        assert_eq!(stats["avg_inbreeding_coefficient"], 0.0, "the dead individual's F must not drag the average");
    }

    #[test]
    fn effective_population_size_follows_wrights_formula_for_a_skewed_sex_ratio() {
        let males: Vec<Individual> = (0..3).map(|_| individual_with("male", true, false, Some(0.0), HashMap::new())).collect();
        let female = individual_with("female", true, false, Some(0.0), HashMap::new());
        let refs: Vec<&Individual> = males.iter().chain(std::iter::once(&female)).collect();
        let stats = compute_genetic_diversity(&refs);
        // Ne = 4*Nm*Nf/(Nm+Nf) = 4*3*1/4 = 3.0
        assert_eq!(stats["effective_population_size"], 3.0);
    }

    #[test]
    fn avg_inbreeding_coefficient_averages_each_living_individuals_own_f() {
        let a = individual_with("male", true, false, Some(0.0), HashMap::new());
        let b = individual_with("female", true, false, Some(0.25), HashMap::new());
        let stats = compute_genetic_diversity(&[&a, &b]);
        assert_eq!(stats["avg_inbreeding_coefficient"], 0.125);
    }
}
