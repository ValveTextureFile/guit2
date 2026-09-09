### VID: 1ed8
### PID: 0037

#### Interface 0:
- HID
- IN: 0x81
- OUT: 0x01

#### HID:
- Vendor-defined usage page 0xFF00
- 64-byte Input report
- 64-byte Output report
- No Report ID

## Fender Protocol

### Questions

- What does the first byte of a HID report mean?
tag

- Is there a length field?
2 byte

- Can one message span multiple HID reports?
yes, 0x33 as start, 0x34 as continuation, and 0x35 as final packet
- How does the amp know when a message ends?
0x35 
- Where does protobuf begin?
for input: 3-63
for output 4-64
- What is the first command Fender Tone sends?
sync, then firmware verison