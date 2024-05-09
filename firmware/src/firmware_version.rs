use late_mate_shared::comms::device_to_host;

pub const fn get_git_firmware_version() -> device_to_host::FirmwareVersion {
    const ENV_GIT_COMMIT_FULL: [u8; 20] = const_str::hex!(env!("VERGEN_GIT_SHA"));

    let mut git_commit = [0u8; 5];
    let mut i = 0;
    while i < 5 {
        git_commit[i] = ENV_GIT_COMMIT_FULL[i];
        i += 1;
    }

    device_to_host::FirmwareVersion {
        git_commit,
        is_dirty: const_str::compare!(==, env!("VERGEN_GIT_DIRTY"), "true"),
    }
}
