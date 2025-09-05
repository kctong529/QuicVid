# Baseline Study: Network Handover Impact on Video Calls

## Overview

This study establishes baseline performance metrics for existing video calling applications during network handovers. The data will provide comparison benchmarks for evaluating the QuicVid prototype's improvements.

## Research Questions

1. **How do current video calling apps respond to network switches?**
2. **What percentage of handovers result in call drops vs. successful recovery?**
3. **How does video/audio quality degrade during network transitions?**
4. **What is the current user experience baseline for network handover scenarios?**

## Objectives

Measure how well different video call applications handle network handoff (cellular ↔ WiFi), focusing on:

- **App response patterns** - How quickly apps detect and respond to network changes
- **Service continuity** - Call survival and recovery time
- **Quality adaptation** - Video/audio degradation and recovery patterns  
- **User perception** - Subjective experience ratings

## Test Applications

Target applications for baseline testing:
- **Zoom** - Enterprise video conferencing (rich diagnostics available)
- **Microsoft Teams** - Business communication platform (moderate diagnostics)
- **WhatsApp** - Consumer messaging with video calls (limited diagnostics)
- **FaceTime** - Apple's video calling (no diagnostics, iOS/macOS only)
- **Google Meet** - Web-based video conferencing (moderate diagnostics)

*Rationale: Covers major platforms across consumer and enterprise segments with varying levels of diagnostic capabilities*

## Key Metrics

### App Response Metrics
- **Detection latency** - Time from network change to first app response
- **Freeze duration** - Time from network switch to any service resumption
- **Recovery time** - Time from network switch to full quality restoration
- **Recovery pattern** - How quality returns (gradual, stepped, quick, or hard restart)

### Call Continuity Metrics
- **Call survival rate** - Percentage of handovers that maintain connection
- **Reconnection behavior** - Automatic vs manual reconnection for dropped calls
- **Connection maintenance** - Apps that preserve session vs. those that restart

### Quality of Experience (QoE) Metrics
- **Video resolution adaptation** - Changes during handoff (e.g., 720p → 360p → 720p)
- **Frame rate changes** - FPS before, during, and after handoff
- **Audio continuity** - Gaps, glitches, or synchronization issues
- **Quality recovery speed** - How quickly full quality returns

### User Experience Metrics
- **Perceived disruption** - 1-5 scale rating of handoff smoothness
- **App feedback quality** - How well apps communicate handover status to users
- **Predictability** - Consistency of app behavior across trials

## Test Environment

### Hardware Requirements
- **Primary test device** - iPhone with WiFi + cellular capability
- **Reference device** - Stable connection device for call partner
- **Consistent platform focus** - Single device type to control variables

### Network Setup
- **Controlled WiFi** - Access point that can be enabled/disabled reliably
- **Cellular connection** - Stable 4G/5G as handoff target
- **Session-based documentation** - Network conditions measured once per testing session

### Software Tools
- **Screen recording** - For precise timing analysis of app responses
- **Speedtest app** - For network condition documentation
- **Network Analyzer** - For iOS network diagnostics where needed
- **App-specific tools** - Built-in diagnostics where available (Zoom/Teams)

## Test Procedure

### Session Setup (Once per testing session)
1. **Document network environment** - Test both WiFi and cellular speeds, signal strength
2. **Assign session ID** - Group all app tests under consistent conditions
3. **Device preparation** - Ensure adequate battery, close background apps
4. **Environmental documentation** - Location, time, weather conditions

### Per-App Test Sequence
1. **Call establishment** - Start call and wait for stable connection (30 seconds)
2. **Baseline recording** - Note initial video quality and app behavior
3. **Trigger handoff** - Disable WiFi (WiFi → Cellular) or enable WiFi (Cellular → WiFi)
4. **Response observation** - Watch for immediate app reactions
5. **Recovery monitoring** - Track quality restoration process
6. **Stabilization verification** - Confirm return to baseline quality

### Critical Timing Points
- **T0: Call stable state** - Established connection before handoff
- **T1: Handoff trigger** - Exact moment of network switch
- **T2: First app response** - When app shows any reaction to network change
- **T3: Service resumption** - When call continues (may be degraded)
- **T4: Quality recovery** - When quality returns to pre-handoff level

## Sample Size and Statistical Considerations

### Trial Requirements
- **5 trials per app per handoff direction** (WiFi→Cellular, Cellular→WiFi)
- **Total: ~50 trials** across 5 apps
- **Session grouping** - Test all apps under same network conditions per session
- **Control variables** - Time of day, device battery level, background activity

### Statistical Analysis Plan
- **App response comparison** - Mean and distribution of detection/recovery times
- **Success rate analysis** - Call survival rates with confidence intervals
- **Quality pattern analysis** - Categorization of recovery behaviors
- **User experience correlation** - Relationship between technical metrics and ratings

## Expected Challenges and Mitigation

### App Diagnostic Variability
**Challenge**: Different apps provide vastly different amounts of diagnostic information  
**Mitigation**: Focus on observable behaviors and timing rather than internal metrics

### Timing Precision
**Challenge**: Manual timing of app responses may be imprecise  
**Mitigation**: Use screen recording for post-analysis verification, focus on meaningful differences

### Network Condition Control
**Challenge**: Ensuring consistent conditions across multiple app tests  
**Mitigation**: Session-based testing with environmental documentation, shorter test windows

### Platform Consistency
**Challenge**: iOS limitations on network monitoring tools  
**Mitigation**: Accept reduced technical precision, focus on user-observable behaviors

## Data Analysis Framework

### App Response Comparison
- **Detection speed analysis** - How quickly each app responds to network changes
- **Recovery strategy classification** - Maintained connection vs. reconnection patterns
- **Quality adaptation patterns** - Bitrate/resolution adjustment behaviors
- **Consistency measurement** - Variability in app responses across trials

### User Experience Analysis
- **Perceived disruption patterns** - Which apps feel smoothest during transitions
- **Recovery predictability** - Apps with consistent vs. variable behaviors
- **Communication effectiveness** - How well apps inform users about handover status

### Baseline Establishment
- **Current performance benchmarks** - Quantitative baselines for QuicVid comparison
- **Problem validation** - Confirmation that handover issues are significant
- **Improvement opportunity identification** - Areas where QUIC migration could excel

## Expected Outcomes

This baseline study should provide:

1. **App response benchmarks** - How current apps handle network handovers
2. **User experience baseline** - Current satisfaction levels with handover experiences
3. **Recovery pattern taxonomy** - Classification of different app behaviors
4. **Research validation** - Evidence that improved handover technology would benefit users

## Timeline

- **Week 1**: Environment setup and methodology refinement
- **Week 2**: Data collection across multiple sessions (50 trials total)
- **Week 3**: Analysis and documentation

*Total effort: ~15 hours as planned in project timeline*

## Success Criteria

The baseline study succeeds if it produces:
- Clear differentiation in app response behaviors
- Quantifiable metrics for QuicVid comparison
- User experience ratings that show room for improvement
- Reproducible methodology for future testing
