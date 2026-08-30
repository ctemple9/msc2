use msc_infrastructure::helper_acquisition::{HelperAcquisitionError, HelperPlatform};
use msc_infrastructure::playit::pinned_playit_release;

#[test]
fn pinned_playit_release_maps_supported_platforms_to_exact_assets() {
    let expected = [
        (
            HelperPlatform::MacosX86_64,
            "playitd-v1.0.10",
            "playitd",
            "91ae745a35aad7a058a9bfb3320d7dc27a54f66a8bb81831360966dd69acc791",
        ),
        (
            HelperPlatform::MacosAarch64,
            "playitd-v1.0.10",
            "playitd",
            "91ae745a35aad7a058a9bfb3320d7dc27a54f66a8bb81831360966dd69acc791",
        ),
        (
            HelperPlatform::LinuxX86_64,
            "v1.0.10",
            "playit-linux-amd64",
            "2df7d9f10227ab312b1ad341853db4e8a8243df5cfcdbae58713a4271711c339",
        ),
        (
            HelperPlatform::LinuxAarch64,
            "v1.0.10",
            "playit-linux-aarch64",
            "4c0db3e7b3a8158e249441c2f0b73f54e83429395890c7b1ca45fd7a6303d763",
        ),
        (
            HelperPlatform::WindowsX86_64,
            "v1.0.10",
            "playit-windows-x86_64-signed.exe",
            "2dbdaad119844cbbc062cc9774b8b462afa5f1b4b7832a9fc5ef4676cae887cf",
        ),
    ];

    for (platform, version, asset_name, sha256) in expected {
        let release = pinned_playit_release(platform).expect("supported Playit pin");
        assert_eq!(release.helper, "playitd");
        assert_eq!(release.version, version);
        assert_eq!(release.assets.len(), 1);
        assert_eq!(release.assets[0].platform, platform);
        assert_eq!(release.assets[0].asset_name, asset_name);
        assert_eq!(release.assets[0].sha256, sha256);
        assert_eq!(release.assets[0].sha256.len(), 64);
    }
}

#[test]
fn windows_arm64_playit_pin_is_explicitly_unsupported() {
    let error = pinned_playit_release(HelperPlatform::WindowsAarch64)
        .expect_err("Windows ARM64 has no pinned Playit asset");

    assert!(matches!(
        error,
        HelperAcquisitionError::ReleaseResolution(message)
            if message.contains("windows-aarch64")
    ));
}
