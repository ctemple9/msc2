use msc_application::java_launch::{
    JavaLaunchFileSystem, PaperLaunchRequest, ValidatedJavaLaunch, build_paper_launch_command,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

struct Fixture {
    input: Value,
    expected: Value,
}

#[derive(Default)]
struct FakeFileSystem {
    files: BTreeSet<PathBuf>,
}

impl JavaLaunchFileSystem for FakeFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        self.files.contains(path)
    }
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/java-launch-paper")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        input: json["input"].clone(),
        expected: json["expected"].clone(),
    }
}

fn request_from(input: &Value) -> PaperLaunchRequest {
    let java = &input["validatedJava"];
    let prefix_arguments = java["prefixArguments"]
        .as_array()
        .expect("validatedJava.prefixArguments")
        .iter()
        .map(|arg| arg.as_str().expect("prefix arg"))
        .collect::<Vec<_>>();

    PaperLaunchRequest::new(
        ValidatedJavaLaunch::new(
            java["executablePath"].as_str().expect("executablePath"),
            prefix_arguments,
        ),
        input["serverDir"].as_str().expect("serverDir"),
        input["paperJarPath"].as_str().expect("paperJarPath"),
        input["minRamGB"].as_f64().expect("minRamGB"),
        input["maxRamGB"].as_f64().expect("maxRamGB"),
        input["extraFlags"].as_str().unwrap_or(""),
    )
}

fn fs_from(input: &Value) -> FakeFileSystem {
    let files = input["existingFiles"]
        .as_array()
        .expect("existingFiles")
        .iter()
        .map(|path| PathBuf::from(path.as_str().expect("existing file path")))
        .collect();
    FakeFileSystem { files }
}

fn assert_fixture(case: &str) {
    let fixture = load(case);
    let fs = fs_from(&fixture.input);
    let request = request_from(&fixture.input);
    let actual = build_paper_launch_command(&fs, &request);

    if let Some(error_contains) = fixture.expected["errorContains"].as_str() {
        let error = actual.expect_err("fixture expected launch construction to fail");
        assert!(
            error.to_string().contains(error_contains),
            "{case}: {error}"
        );
        return;
    }

    let command = actual.expect("fixture expected launch construction to succeed");
    assert_eq!(
        command.executable_path,
        PathBuf::from(
            fixture.expected["executablePath"]
                .as_str()
                .expect("expected executablePath")
        )
    );
    assert_eq!(
        command.working_directory,
        PathBuf::from(
            fixture.expected["workingDirectory"]
                .as_str()
                .expect("expected workingDirectory")
        )
    );

    let expected_arguments = fixture.expected["arguments"]
        .as_array()
        .expect("expected arguments")
        .iter()
        .map(|arg| arg.as_str().expect("expected argument").to_string())
        .collect::<Vec<_>>();
    assert_eq!(command.arguments, expected_arguments);

    if let Some(contains) = fixture.expected["argumentsContains"].as_array() {
        for expected in contains {
            let expected = expected.as_str().expect("argumentsContains item");
            assert!(
                command.arguments.iter().any(|arg| arg == expected),
                "{case}: expected argument {expected:?} in {:?}",
                command.arguments
            );
        }
    }

    if fixture.expected["argumentsHaveNoEmptyString"]
        .as_bool()
        .unwrap_or(false)
    {
        assert!(
            command.arguments.iter().all(|arg| !arg.is_empty()),
            "{case}: arguments must not contain empty strings: {:?}",
            command.arguments
        );
    }
}

#[test]
fn java_launch_paper_jar_command() {
    assert_fixture("paper-jar-command");
}

#[test]
fn java_launch_paper_uses_working_directory_and_jar_basename() {
    assert_fixture("uses-working-directory-and-jar-basename");
}

#[test]
fn java_launch_paper_env_prefix_arguments_precede_jvm_flags() {
    assert_fixture("env-prefix-arguments-precede-jvm-flags");
}

#[test]
fn java_launch_paper_heap_flags_round_fractional_gb_to_mb() {
    assert_fixture("heap-flags-round-fractional-gb-to-mb");
}

#[test]
fn java_launch_paper_sandbox_suppress_flags_present() {
    assert_fixture("sandbox-suppress-flags-present");
}

#[test]
fn java_launch_paper_extra_flags_are_split_into_arguments() {
    assert_fixture("extra-flags-are-split-into-arguments");
}

#[test]
fn java_launch_paper_empty_extra_flags_are_omitted() {
    assert_fixture("empty-extra-flags-are-omitted");
}

#[test]
fn java_launch_paper_missing_jar_fails_before_spawn() {
    assert_fixture("missing-jar-fails-before-spawn");
}
