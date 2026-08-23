#[test]
fn ammonia_filters_urls_set_by_svg_animation_elements() {
    // mdo's production policy rejects SVG entirely. Configure a test-only
    // builder that permits the vulnerable elements so this regression test
    // exercises RUSTSEC-2026-0213 rather than relying on that broader policy.
    let payload = r#"
        <svg>
            <a>
                <set attributeName="href" to="javascript:alert('SET_XSS')"></set>
                <set attributeName="href" to="https://example.com/set"></set>
                <animate attributeName="href" values="javascript:alert('ANIMATE_XSS')"></animate>
                <animate attributeName="href" values="https://example.com/animate"></animate>
            </a>
        </svg>
    "#;

    let sanitized = ammonia::Builder::default()
        .add_tags(&["svg", "a", "set", "animate"])
        .add_tag_attributes("set", &["attributeName", "to"])
        .add_tag_attributes("animate", &["attributeName", "values"])
        .clean(payload)
        .to_string();

    assert!(!sanitized.contains("javascript:"), "{sanitized}");
    assert!(!sanitized.contains("SET_XSS"), "{sanitized}");
    assert!(!sanitized.contains("ANIMATE_XSS"), "{sanitized}");
    assert!(sanitized.contains("https://example.com/set"), "{sanitized}");
    assert!(
        sanitized.contains("https://example.com/animate"),
        "{sanitized}"
    );
}
