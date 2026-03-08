#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// Returns the vendor number of the specified display's monitor.
    ///
    /// # Arguments
    ///
    /// * `display` - The identifier of the display to be accessed.
    ///
    /// # Returns
    ///
    /// A vendor number for the monitor associated with the specified display,
    /// or a constant to indicate an exception -- see the discussion below.
    ///
    /// # Discussion
    ///
    /// This function uses I/O Kit to identify the monitor associated with the
    /// specified display.
    ///
    /// There are three cases:
    ///
    /// - If I/O Kit can identify the monitor, the vendor ID is returned.
    /// - If I/O Kit cannot identify the monitor, kDisplayVendorIDUnknown is
    ///   returned.
    /// - If there is no monitor associated with the display, 0xFFFFFFFF is
    ///   returned.
    pub fn CGDisplayVendorNumber(display: u32) -> u32;

    /// Returns the model number of a display monitor.
    ///
    /// # Arguments
    ///
    /// * `display` - The identifier of the display to be accessed.
    ///
    /// # Returns
    ///
    /// A model number for the monitor associated with the specified display,
    /// or a constant to indicate an exception -- see the discussion below.
    ///
    /// # Discussion
    ///
    /// This function uses I/O Kit to identify the monitor associated with the
    /// specified display.
    ///
    /// There are three cases:
    ///
    /// - If I/O Kit can identify the monitor, the product ID code for the
    ///   monitor is returned.
    /// - If I/O Kit cannot identify the monitor, kDisplayProductIDGeneric is
    ///   returned.
    /// - If no monitor is connected, a value of 0xFFFFFFFF is returned.
    pub fn CGDisplayModelNumber(display: u32) -> u32;

    /// Returns the serial number of a display monitor.
    ///
    /// # Arguments
    ///
    /// * `display` - The identifier of the display to be accessed.
    ///
    /// # Returns
    ///
    /// A serial number for the monitor associated with the specified display,
    /// or a constant to indicate an exception -- see the discussion below.
    ///
    /// # Discussion
    ///
    /// This function uses I/O Kit to identify the monitor associated with the
    /// specified display.
    ///
    /// If I/O Kit can identify the monitor:
    ///
    /// - If the manufacturer has encoded a serial number for the monitor, the
    ///   number is returned.
    /// - If there is no encoded serial number, 0x00000000 is returned.
    ///
    /// If I/O Kit cannot identify the monitor:
    ///
    /// - If a monitor is connected to the display, 0x00000000 is returned.
    /// - If no monitor is connected to the display hardware, 0xFFFFFFFF is
    ///   returned.
    ///
    /// Note that a serial number is meaningful only in conjunction with a
    /// specific vendor and product or model.
    pub fn CGDisplaySerialNumber(display: u32) -> u32;
}
