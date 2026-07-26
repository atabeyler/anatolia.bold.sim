use sim_core::create_genome;

#[test]
fn language_related_loci_are_mapped_to_chromosome_7_for_future_linkage_modeling() {
    let genome = create_genome(None);

    for locus_id in ["FOXP2_01", "CNTNAP2_01", "RELN_01"] {
        assert_eq!(
            genome[locus_id].chromosome.as_deref(),
            Some("7"),
            "{locus_id} should remain on chromosome 7 so future linkage/crossing-over code has a stable map"
        );
    }
}

#[test]
fn x_linked_behavior_locus_is_explicitly_on_x_chromosome() {
    let genome = create_genome(None);
    assert_eq!(genome["MAOA_01"].chromosome.as_deref(), Some("X"));
}

#[test]
fn every_generated_locus_has_a_chromosome_annotation() {
    let genome = create_genome(None);
    let missing: Vec<_> = genome
        .iter()
        .filter_map(|(locus_id, locus)| locus.chromosome.is_none().then_some(locus_id.clone()))
        .collect();
    assert!(missing.is_empty(), "loci missing chromosome annotations: {missing:?}");
}
