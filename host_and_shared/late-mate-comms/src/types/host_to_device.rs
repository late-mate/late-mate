use postcard::experimental::max_size::MaxSize;

#[derive(Debug, Eq, PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, MaxSize)]
pub enum HostToDevice {
    GetStatus,
    // reset needs reports too to make sure that the light level comes back to the baseline
    SendHidEvent { hid_event: u8, max_duration_ms: u32 },
    MeasureBackground { duration_ms: u32 },
    // note: enum has to allocate space for the largest member, so it can't be included
    //       in the enum itself (regardless of allocation issues)
    UpdateFirmware { length: u32 },
}
