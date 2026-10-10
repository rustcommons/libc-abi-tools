use libc_abi_tools::{
    glibc::{self, Kind, Snapshot, Symbol},
    matrix::{Expectation, Matrix},
};
use std::collections::BTreeMap;

fn fixture(release: &str, entries: &[(&str, &str, &str, Kind)]) -> Snapshot {
    let mut libraries: BTreeMap<String, Vec<Symbol>> = glibc::LIBRARIES
        .iter()
        .map(|lib| ((*lib).into(), Vec::new()))
        .collect();
    for (library, name, version, kind) in entries {
        libraries.get_mut(*library).unwrap().push(Symbol {
            name: (*name).into(),
            version: version.parse().unwrap(),
            kind: *kind,
        });
    }
    Snapshot {
        release: release.parse().unwrap(),
        libraries,
    }
}

#[test]
fn membership_handles_removals_reintroductions_migrations_and_unknowns() {
    let matrix = Matrix::from_snapshots([
        fixture(
            "2.17",
            &[
                ("pthread", "old", "2.2.5", Kind::Function),
                ("c", "view", "2.2.5", Kind::Object(8)),
            ],
        ),
        fixture(
            "2.22",
            &[
                ("c", "old", "2.2.5", Kind::Function),
                ("c", "view", "2.2.5", Kind::Object(16)),
            ],
        ),
        fixture(
            "2.34",
            &[
                ("pthread", "old", "2.2.5", Kind::Function),
                ("c", "tls", "2.34", Kind::Tls(4)),
            ],
        ),
    ])
    .unwrap();
    assert_eq!(
        matrix.expect(
            "2.22".parse().unwrap(),
            "pthread",
            "old",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Reject
    );
    assert_eq!(
        matrix.expect(
            "2.34".parse().unwrap(),
            "pthread",
            "old",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Accept(Kind::Function)
    );
    assert_eq!(
        matrix.expect(
            "2.22".parse().unwrap(),
            "c",
            "old",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Accept(Kind::Function)
    );
    assert_eq!(
        matrix.expect(
            "2.22".parse().unwrap(),
            "c",
            "view",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Accept(Kind::Object(16))
    );
    assert_eq!(
        matrix.expect(
            "2.21".parse().unwrap(),
            "c",
            "old",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Unknown
    );
    assert_eq!(
        matrix.expect(
            "2.16".parse().unwrap(),
            "c",
            "old",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Unknown
    );
    assert_eq!(
        matrix.expect(
            "2.22".parse().unwrap(),
            "anl",
            "old",
            "2.2.5".parse().unwrap()
        ),
        Expectation::Unknown
    );
    let diff = matrix
        .diff_tsv("2.17".parse().unwrap(), "2.22".parse().unwrap(), None)
        .unwrap();
    assert!(diff.contains("added\tc\told\tGLIBC_2.2.5\tabsent\tF"));
    assert!(diff.contains("removed\tpthread\told\tGLIBC_2.2.5\tF\tabsent"));
    assert!(diff.contains("changed\tc\tview\tGLIBC_2.2.5\tD:8\tD:16"));
    assert!(matrix
        .diff_tsv("2.16".parse().unwrap(), "2.22".parse().unwrap(), None)
        .is_err());
}

#[test]
fn rejects_incomplete_duplicate_or_invalid_evidence() {
    assert!(Matrix::from_snapshots([]).is_err());
    let snap = fixture("2.17", &[("c", "x", "2.17", Kind::Function)]);
    assert!(Matrix::from_snapshots([snap.clone(), snap]).is_err());
    assert!(
        Matrix::from_snapshots([fixture("2.17", &[("c", "x", "2.22", Kind::Function)])]).is_err()
    );
    assert!(Matrix::from_snapshots([fixture(
        "2.17",
        &[("c", "bad\tname", "2.17", Kind::Function)]
    )])
    .is_err());
    assert!(Matrix::from_snapshots([fixture(
        "2.17",
        &[
            ("c", "x", "2.17", Kind::Function),
            ("c", "x", "2.17", Kind::Function)
        ]
    )])
    .is_err());
    let mut snap = fixture("2.17", &[]);
    snap.libraries.remove("c");
    assert!(Matrix::from_snapshots([snap]).is_err());
}

#[test]
fn historical_namespace_is_not_first_observed_release_or_default_binding() {
    let a = fixture(
        "2.22",
        &[
            ("c", "fmemopen", "2.2.5", Kind::Function),
            ("c", "fmemopen", "2.22", Kind::Function),
        ],
    );
    let b = fixture("2.17", &[("c", "fmemopen", "2.2.5", Kind::Function)]);
    let reverse = Matrix::from_snapshots([a.clone(), b.clone()]).unwrap();
    let forward = Matrix::from_snapshots([b, a]).unwrap();
    assert_eq!(forward.to_tsv(None), reverse.to_tsv(None));
    assert_eq!(forward.changes_tsv(), reverse.changes_tsv());
    assert!(forward
        .to_tsv(None)
        .contains("c\tfmemopen\tGLIBC_2.2.5\t2.17\tF\tF"));
    assert!(forward
        .to_tsv(None)
        .contains("c\tfmemopen\tGLIBC_2.22\t2.22\tabsent\tF"));
}

#[cfg(feature = "bundled-glibc-data")]
#[test]
fn pinned_matrix_classifies_version_boundaries_and_real_library_migration() {
    let matrix = Matrix::bundled().unwrap();
    for minor in 17..=44 {
        let release = format!("2.{minor}").parse().unwrap();
        let old = matrix.expect(release, "c", "fmemopen", "2.2.5".parse().unwrap());
        assert_eq!(old, Expectation::Accept(Kind::Function));
        assert_eq!(
            matrix
                .expect(release, "c", "fmemopen", "2.22".parse().unwrap())
                .label(),
            if minor < 22 { "reject" } else { "accept" }
        );
        assert_eq!(
            matrix
                .expect(release, "c", "memfd_create", "2.27".parse().unwrap())
                .label(),
            if minor < 27 { "reject" } else { "accept" }
        );
    }
    assert_eq!(
        matrix
            .expect(
                "2.33".parse().unwrap(),
                "pthread",
                "pthread_create",
                "2.2.5".parse().unwrap()
            )
            .label(),
        "accept"
    );
    assert_eq!(
        matrix
            .expect(
                "2.34".parse().unwrap(),
                "pthread",
                "pthread_create",
                "2.2.5".parse().unwrap()
            )
            .label(),
        "reject"
    );
    assert_eq!(
        matrix
            .expect(
                "2.34".parse().unwrap(),
                "c",
                "pthread_create",
                "2.2.5".parse().unwrap()
            )
            .label(),
        "accept"
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_libc-abi"))
        // No --data: the CLI reads the bundled glibc-abilists data.
        .args(["glibc", "expect"])
        .args([
            "--release",
            "2.16",
            "--library",
            "c",
            "--symbol",
            "fmemopen",
            "--version",
            "2.2.5",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"unknown\n");
}
