use player_services::ShaderFile;
use std::path::Path;
#[test]
fn selected_shader_is_bounded_validated_and_preserves_punctuation() {
    assert!(ShaderFile::new(Path::new("relative.glsl")).is_err());
    let folder = std::env::current_dir()
        .unwrap()
        .join("../../results/local")
        .join(format!("shader-test-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("selected shader,with;punctuation.glsl");
    let source = "//!HOOK MAIN\n//!BIND HOOKED\nvec4 hook() { return HOOKED_tex(HOOKED_pos); }\n";
    std::fs::write(&path, source).unwrap();
    let shader = ShaderFile::new(&path).unwrap();
    assert_eq!(std::fs::read_to_string(shader.path()).unwrap(), source);
    std::fs::write(&path, "ordinary text").unwrap();
    assert!(ShaderFile::new(&path).unwrap_err().contains("HOOK"));
    std::fs::write(&path, b"//!HOOK MAIN\n\0").unwrap();
    assert!(ShaderFile::new(&path).is_err());
    std::fs::write(&path, [0xff]).unwrap();
    assert!(ShaderFile::new(&path).is_err());
    std::fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(ShaderFile::new(&path).unwrap_err().contains("1 MiB"));
    assert!(ShaderFile::new(&folder).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(ShaderFile::new(&path).is_err());
    std::fs::remove_dir(folder).unwrap();
}
