# QuicVid

**Research prototype exploring seamless video call handover using QUIC connection migration**

This is a bachelor final project at Aalto University investigating whether QUIC's connection migration feature can solve the frustrating problem of dropped video calls when switching between networks (WiFi ↔ mobile data).

## 🎯 Project Goals

Video calls today break when you switch networks. This project aims to build a prototype that demonstrates:

- **Seamless handover** - Calls continue when switching from WiFi to mobile data
- **Sub-second migration** - Network transitions complete in <500ms  
- **Real-world validation** - User testing to measure actual experience improvement
- **Open research** - Reproducible methodology for future QUIC + multimedia research

## 🧪 Research Questions

1. **Can QUIC migration work for real-time video?** Existing research shows QUIC benefits for streaming, but migration during active calls remains unexplored
2. **How much does user experience improve?** Measuring both technical metrics and user satisfaction compared to traditional approaches
3. **What are the implementation challenges?** Documenting practical issues when combining real-time media with connection migration

## 🏗️ Planned Architecture

```
Video Calling App (Qt)
├── Media Pipeline (FFmpeg) - H.264 video + Opus audio
├── QUIC Transport (quiche) - Connection migration capability  
├── Handover Logic - Network change detection and migration triggers
└── Testing Framework - Automated and user testing tools
```

## 📅 Development Timeline

- **Phase 0.5 (Sep)**: User tests for problem baseline
- **Phase 1** (Sep-Oct): QUIC foundation with basic migration
- **Phase 2** (Nov): Video streaming integration  
- **Phase 3** (Dec-Jan): Seamless handover implementation
- **Phase 4** (Feb): Performance evaluation and user testing
- **Phase 5** (Mar): Documentation

## 🔍 Current Status

🚧 **In Development** - Setting up foundation and QUIC integration

Follow along as this research unfolds! The goal is to demonstrate that protocol-level improvements can solve real user problems.

## 📊 Success Metrics (Targets)

| Goal | Target | Why It Matters |
|------|--------|----------------|
| Handover time | <500ms | Below user perception threshold |
| Call survival rate | >95% | Reliable enough for real use |
| User preference | >80% prefer over baseline | Measurable UX improvement |

## 🛠️ Tech Stack

- **QUIC:** quiche (Cloudflare's Rust implementation)
- **Media:** FFmpeg for video/audio processing
- **GUI:** Qt 6 for cross-platform interface
- **Platform:** Linux primary, macOS secondary
- **Testing:** Controlled network environments + user studies

## 📚 Why This Matters

**Technical Impact:** First real-world demonstration of QUIC migration for multimedia

**User Impact:** Could eliminate one of the most frustrating aspects of video calling

**Research Impact:** Bridges the gap between networking protocol research and actual user experience

## 🤝 Following This Project

This is active research! If you're interested in:
- **QUIC protocol development**
- **Real-time multimedia systems** 
- **Network handover solutions**
- **User experience research**

Watch this space for updates, findings, and open questions.

## 📄 Future Plans

If successful, this prototype could lead to:
- Integration with existing video calling platforms
- Mobile app development
- Production deployment studies
- Follow-up research on multi-party scenarios

