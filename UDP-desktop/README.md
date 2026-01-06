# Application Functionality Report: UDP Image Streamer

This C++ Qt application is a minimal, integrated proof-of-concept for real-time image streaming using the User Datagram Protocol (UDP), demonstrating both the server (sender) and client (receiver) roles within a single window. This serves as a demonstration of integrating GUI components with UDP network communication.

## Core Purpose

The application creates a high-frequency, low-latency stream of an interactive drawing canvas. The server continuously captures the drawing, compresses it, and sends it as a UDP datagram to the client, which immediately reconstructs and displays the image.

## Server (Sender) Functionality

The server component is responsible for generating and transmitting the stream:

1. **Input Source:** The user draws on a custom DrawingWidget. This canvas serves as the source "video" feed.

2. **Timing:** A QTimer periodically triggers the sendFrame() slot (at 20 FPS), ensuring continuous updates.

3. **Transmission Preparation:** The current state of the drawing canvas (QImage) is retrieved, compressed into a JPEG format (QByteArray) for bandwidth efficiency, and sent via QUdpSocket to a specific port (12345) on the local machine (127.0.0.1).

## Client (Receiver) Functionality

The client component is responsible for network reception and display:

1. **Listening:** A separate QUdpSocket is bound to the target port (12345) to listen for incoming data.

2. **Processing:** Upon receiving a datagram (readyRead() signal), the client reads the compressed QByteArray.

3. **Display:** The byte array is decompressed and loaded into a QPixmap object, which is then immediately displayed in the client's viewer panel (clientImageLabel).

## Project Context: QUIC Foundation

The use of UDP is foundational to this experiment. While the ultimate project goal is focused on the QUIC protocol, which operates over UDP, this initial application confirms reliable, basic datagram transmission for real-time streaming. This setup allows for simple testing of network communication performance and latency characteristics before implementing the advanced features (such as reliable delivery, stream multiplexing, and congestion control) provided by QUIC.
