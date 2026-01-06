# Hardware Mouse Telemetry over QUIC: Demonstrating Session Persistence in Mininet Environments

This system is a high-performance **Telemetry Pipeline** designed to demonstrate QUIC’s resilience to network changes. It uses a **Client-Server** model to stream hardware input over a simulated network.

### Core Components

* **The Client (Producer):** Captures raw relative movement from `/dev/input/mice` at **60Hz**. It accumulates these into a coordinate state () and transmits them as **QUIC Datagrams**.
* **The Network (Mininet):** Provides a virtual environment where the Client's IP address can be changed mid-stream to trigger **Connection Migration**.
* **The Server (Consumer):** Maintains a persistent session via **Connection IDs**. It maps coordinates to a **100x36 ASCII grid** and renders them in real-time using ANSI escape codes.

### Key Mechanisms

1. **Unreliable Datagrams:** Uses fire-and-forget packets to eliminate **Head-of-Line blocking**, ensuring that a lost packet never causes a lag spike in the cursor movement.
2. **Session Persistence:** Unlike TCP, which is bound to an IP address, this system uses QUIC's **Connection ID** to keep the session alive even when the underlying network path changes.
3. **Low-Latency Rendering:** Employs "ghosting" logic to erase and redraw the cursor at 60fps without flickering or full-screen refreshes.

## What is involved in this Program?

The program is a **Real-Time Distributed Input System**. It is comprised of three distinct technical layers:

### The Hardware Layer (Input Capture)

Instead of using a high-level GUI library, the client interacts directly with the Linux kernel via `/dev/input/mice`.

* **Mechanism:** It reads 3-byte hardware packets containing relative movement (deltas).
* **Processing:** These deltas are accumulated into a coordinate state () and clamped to a logical boundary. This represents a "Raw Input" architecture used in high-performance gaming and remote desktop software.

### The Transport Layer (QUIC + Datagrams)

This is the core of the project. We use the **QUIC protocol** (standardized as RFC 9000) instead of TCP.

* **Unreliable Datagrams:** We use the QUIC Datagram extension. If a mouse coordinate is lost, the system doesn't wait to retransmit it (which would cause a "jump" or "lag spike"). It simply waits for the next one.
* **Encryption:** Every packet is encrypted using TLS 1.3, which is baked directly into the QUIC header.

### The Presentation Layer (ANSI Visualizer)

The server uses **ANSI Escape Sequences** to turn a standard text terminal into a 2D canvas.

* **State Management:** The server tracks a "ghost" position to erase the previous cursor, allowing for a 60Hz flicker-free update without redrawing the entire screen.

---

## Why is this significant?

This setup demonstrates two revolutionary concepts in modern networking: **Connection Migration** and **Zero Head-of-Line Blocking**.

### A. Connection Migration (The "IP-Agnostic" Session)

In traditional networking (TCP), a connection is tied to your IP address. If you move from Wi-Fi to 4G, your IP changes and your Zoom call or SSH session drops.

* **Significance:** Your program proves that by using a **Connection ID (CID)**, the session is tied to the *identity* of the host, not its location.
* **Demonstration:** When you run `ifconfig` in Mininet, you are performing a "hard handover." In TCP, this would crash the client. In your program, the cursor keeps moving because the server sees the CID and says, "I know who you are, even if your address is new."

### B. Eliminating Head-of-Line (HOL) Blocking

In TCP, if Packet #1 is lost, Packet #2 must wait in the buffer until Packet #1 is retransmitted. This is disastrous for real-time data like mouse movement or VR head-tracking.

* **Significance:** By using **QUIC Datagrams**, your program implements "Unordered Delivery."
* **Impact:** If the network gets congested during your Mininet test, the mouse might skip slightly, but it will never "freeze and then fast-forward" (the classic TCP lag behavior).

### C. The Power of Rust for Systems Networking

Using Rust ensures that this high-frequency 60Hz streaming happens with **zero-cost abstractions**.

* The memory safety of Rust prevents buffer overflows when parsing the hardware mouse bytes, and the `tokio` runtime allows the server to handle hundreds of these mouse streams simultaneously without slowing down.
