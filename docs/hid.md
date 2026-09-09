
## Definitions
**HID** --> Human Interface Device
**HID Report Descriptor** --> Describes the layout of the data packet of *HID reports*
**HID Report** --> A structured block of data sent between the host and HID device.
	There are 3 type of reports:
		Input Report: Device → Host
		Output Report: Host → Device
		Feature Report: Configuration or state data that the host can read or write.


### Searching for USB HID devices in Linux
___
Using `lsusb`, we can find what bus, device number, and ID a HID device can have:
```
~ ? ❯ lsusb
Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 001 Device 002: ID 2109:2817 VIA Labs, Inc. USB2.0 Hub             
Bus 001 Device 003: ID 0781:55ae SanDisk Corp. Extreme 55AE
Bus 001 Device 004: ID 04f2:b7b6 Chicony Electronics Co., Ltd Integrated Camera
Bus 001 Device 005: ID 05e3:0608 Genesys Logic, Inc. Hub
Bus 001 Device 006: ID 048d:c997 Integrated Technology Express, Inc. ITE Device(8258)
Bus 001 Device 007: ID 291a:8365 Anker                   Anker USB-C Hub Device 
Bus 001 Device 008: ID 328f:0073 EMEET HD Webcam eMeet C950
Bus 001 Device 009: ID 27c6:659a Shenzhen Goodix Technology Co.,Ltd. Goodix USB2.0 MISC
Bus 001 Device 011: ID 17ef:f006 Lenovo Idea5003
Bus 001 Device 012: ID 8087:0033 Intel Corp. AX211 Bluetooth
Bus 001 Device 013: ID 1b1c:1b5e Corsair Harpoon Wireless Mouse
Bus 001 Device 019: ID 1038:1646 SteelSeries ApS Apex Pro TKL Wireless Gen 3
Bus 001 Device 020: ID 1ed8:0037 FENDER MUSICAL INSTRUMENTS CORPORATION Mustang LT 25
Bus 002 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 002 Device 002: ID 1058:2630 Western Digital Technologies, Inc. Game Drive
Bus 002 Device 003: ID 2109:0817 VIA Labs, Inc. USB3.0 Hub             
Bus 002 Device 004: ID 0b95:1790 ASIX Electronics Corp. AX88179 Gigabit Ethernet
Bus 003 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 004 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
```
For this instance (and purpose), the focus will be on `FENDER MUSICAL INSTRUMENTS CORPORATION Mustang LT 25`, my guitar amp.

We can use `udevadm` to grab more information on the HID. Using a bash for loop, we can loop through `hidraw` devices in `/dev` and find the HID id for the LT25:
```
for dev in /dev/hidraw*; do
	echo "=== $dev ===";
	udevadm info -q property -n "$dev" 2>/dev/null |
	grep -E 'ID_VENDOR_ID|ID_MODEL_ID|ID_VENDOR=|ID_MODEL=';
done
```

If you already know the device, and want more information than just the ids listed, use `sudo udevadm info /dev/hidraw{id}`

## USB Device Architecture

Before diving deeper into HID specifics, it helps to understand where HID sits inside the broader USB descriptor hierarchy. USB devices are organized in a tree:

```
USB Device
│
├── Endpoint 0
│   └── Control / enumeration / descriptors
│
└── Configuration
    └── Interface
        └── Other endpoints
```

### USB Device

The physical device itself (e.g. the Mustang LT25 amp). A device has a **Vendor ID (VID)** and **Product ID (PID)** - this is the `1ed8:0037` pair you saw in `lsusb`. A device can support one or more configurations, though most devices only implement one.

### Configuration

A **configuration** describes a specific power/operational state and bundles together one or more interfaces. Only one configuration can be active at a time. Most consumer devices (mice, keyboards, amps) only expose a single configuration, so this layer is often invisible in practice - but it exists in the descriptor tree.

### Interface

An **interface** represents a logical function of the device. A single physical USB device can expose multiple interfaces - for example, a webcam might have one interface for video and one for its built-in microphone. Each interface has a **class**, **subclass**, and **protocol** - HID devices declare `bInterfaceClass = 0x03` (HID class). This is how the OS knows to load the HID driver for that interface rather than, say, a mass-storage or audio driver.

### Endpoint

An **endpoint** is the actual communication channel/buffer used to move data between host and device. Endpoints are unidirectional and have a type: **Control**, **Interrupt**, **Bulk**, or **Isochronous**. HID devices almost always use **Interrupt** endpoints for their input/output reports because they need low-latency, polled communication (rather than the high-throughput but less time-sensitive Bulk transfers used by, say, storage devices).

### IN / OUT

Endpoint direction is defined from the **host's perspective**:

- **IN** - data flows _from_ the device _to_ the host (e.g. mouse movement, button state, guitar amp input events)
- **OUT** - data flows _from_ the host _to_ the device (e.g. host tells amp to switch presets, host sets keyboard LED state)

A device typically has at least one Interrupt IN endpoint (to send input reports) and optionally an Interrupt OUT endpoint (to receive output reports), though output/feature reports can also travel over the Control endpoint (endpoint 0) via `SET_REPORT`/`GET_REPORT` requests instead of a dedicated OUT endpoint.

---

## HID Layer

Once an interface declares itself as HID class, the HID specification defines how that interface describes and exchanges HID reports.

### HID

**Human Interface Device** - the USB device class designed for devices that send/receive relatively small, structured data packets, originally meant for keyboards/mice/joysticks but now used far more broadly (as with the Mustang LT25) because the HID class doesn't require a custom driver - the OS's generic HID driver can talk to any compliant device.

### Report

The actual data packet exchanged between host and device. As covered:

- **Input Report** - device → host (button presses, sensor data, knob positions)
- **Output Report** - host → device (LED state, preset changes)
- **Feature Report** - configuration data, readable and/or writable, often used for things like DPI settings, calibration, or device-specific configuration parameters

### Report Descriptor

A special binary blob the device supplies (retrieved via `GET_DESCRIPTOR` on the control endpoint) that acts as a **schema** for its reports. It tells the host exactly how to interpret the raw bytes in each report — which bits/bytes correspond to which control, what range of values are valid, and what those values _mean_. Without the report descriptor, a report is just an opaque array of bytes.

You can dump a device's report descriptor with:

```
sudo usbhid-dump -a 001:019 -e descriptor
```

or, using hidraw:

```
sudo hidrd-convert -o spec /sys/class/hidraw/hidraw{id}/device/report_descriptor
```

### Usage Page

A high-level category grouping of controls, defined in the HID Usage Tables spec. Examples:

- `0x01` - Generic Desktop (mice, joysticks, keyboards)
- `0x0C` - Consumer (media keys, volume)
- `0x08` - LEDs
- Vendor-defined pages exist too (`0xFF00`-`0xFFFF` range) - often used by devices like the Mustang LT25 for proprietary, non-standard functionality (amp presets, tone settings, etc. aren't part of any standard HID usage table).

### Usage

A specific control _within_ a usage page. For example, under the Generic Desktop page (`0x01`):

- `0x30` - X axis
- `0x31` - Y axis
- `0x90` - D-pad Up

A Usage Page + Usage pair together fully identifies what a piece of data represents (e.g. "Generic Desktop / X axis" = mouse X movement).

### Collection

A grouping construct within the report descriptor used to organize related usages together - similar to a struct or a logical container. Common types:

- **Application** - top-level collection representing a complete, standalone function (e.g. "this whole descriptor is a mouse")
- **Physical** - groups usages that represent the same physical point of measurement (e.g. all axes on one joystick)
- **Logical** - groups usages related in a data-processing sense, without a physical relationship
- **Report** / **Named Array** / **Usage Switch** - more specialized grouping types

Collections can be nested, letting a single interface describe complex composite devices (e.g. a gamepad with joystick + button collections all in one report descriptor).

### Report Size

The number of **bits** each individual data field within the report occupies. E.g. a button might be a single bit (`Report Size = 1`), while an axis value might be 8 or 16 bits.

### Report Count

The number of fields of that report size that repeat consecutively. If `Report Size = 1` and `Report Count = 8`, that's 8 individual 1-bit fields in a row - commonly used for 8 button states packed into a single byte.

Together, `Report Size × Report Count` tells you exactly how many bits a given block of the report consumes.

### Report ID

When a device sends multiple _distinct_ report structures over the same endpoint (e.g. one report layout for button data, another for axis data), each report is prefixed with a **Report ID** byte so the host knows which schema (from the report descriptor) to apply when parsing the incoming bytes. If a device only has one report type, Report IDs are often omitted entirely (defaulting to ID 0, with no prefix byte).

### hidraw

The Linux kernel interface (`/dev/hidraw0`, `/dev/hidraw1`, etc.) that exposes **raw, unparsed** HID reports directly to userspace - bypassing the kernel's higher-level HID input subsystem (`evdev`) that normally translates HID data into standardized input events for things like mice and keyboards.

`hidraw` is the tool of choice when:

- You're dealing with a non-standard/vendor-defined HID device (like the Mustang LT25) that the generic input subsystem doesn't know how to interpret meaningfully
- You need to read/write **Feature** or **Output** reports directly
- You're reverse-engineering a device's protocol from its report descriptor

You can read/write raw reports from userspace via simple file I/O on `/dev/hidraw{id}`, or use `ioctl()` calls (`HIDIOCGRDESCSIZE`, `HIDIOCGRDESC`, `HIDIOCSFEATURE`, `HIDIOCGFEATURE`, etc.) for descriptor retrieval and Feature report access specifically.