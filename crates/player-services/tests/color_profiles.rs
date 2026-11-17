use player_services::DisplayProfile;
use std::path::{Path, PathBuf};
#[test]
fn display_profile_selection_rejects_malformed_or_non_display_resources() {
    assert!(DisplayProfile::new(Path::new("relative.icc")).is_err());
    let original = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../player-render/tests/fixtures/srgb.icc"),
    )
    .unwrap();
    let folder = std::env::current_dir()
        .unwrap()
        .join("../../results/local")
        .join(format!("profile-test-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("display profile,selected.icc");
    std::fs::write(&path, &original).unwrap();
    assert_eq!(
        std::fs::read(DisplayProfile::new(&path).unwrap().path()).unwrap(),
        original
    );
    for (offset, replacement) in [
        (36, b"oops".as_slice()),
        (12, b"prtr".as_slice()),
        (16, b"CMYK".as_slice()),
        (0, &[0, 0, 0, 1]),
        (128, &[255, 255, 255, 255]),
        (140, &[255, 255, 255, 255]),
    ] {
        let mut broken = original.clone();
        broken[offset..offset + 4].copy_from_slice(replacement);
        std::fs::write(&path, broken).unwrap();
        assert!(
            DisplayProfile::new(&path).is_err(),
            "Corrupt header/table at {offset} must be rejected"
        );
    }
    std::fs::write(&path, &original[..100]).unwrap();
    assert!(DisplayProfile::new(&path).is_err());
    std::fs::write(&path, vec![0u8; 4 * 1024 * 1024 + 1]).unwrap();
    assert!(DisplayProfile::new(&path).unwrap_err().contains("4 MiB"));
    assert!(DisplayProfile::new(&folder).is_err());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(folder).unwrap();
}
