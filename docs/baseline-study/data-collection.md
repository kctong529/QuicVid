# Baseline Study: Data Collection Guide

## CSV Data Template

Use this standardized format to record data for each test trial. Save as `baseline-data.csv` in the `docs/baseline-study/results/raw-data/` directory.

### CSV Headers

```csv
Trial,App,Device_OS,Network_Switch,Start_Time,Network_Session_ID,Freeze_Duration_s,Recovery_Time_s,Call_Drop_YN,Video_Resolution_Before,Video_Resolution_During,Video_Resolution_After,Frame_Rate_Before,Frame_Rate_During,Frame_Rate_After,Audio_Issues_YN,Audio_Issue_Notes,App_Reconnection_YN,Quality_Recovery_Pattern,User_Rating_1to5,Notes
```

### Example Data Rows

```csv
1,Zoom,iPhone_13_iOS_16.1,WiFi_to_Cellular,12:05:10,Session_A,1.2,3.5,N,720p,360p,720p,30,15,30,Y,1s audio glitch,N,Gradual_improvement,4,App showed connection warning but maintained call
2,WhatsApp,iPhone_13_iOS_16.1,WiFi_to_Cellular,12:12:42,Session_A,2.0,4.1,N,480p,240p,480p,20,10,20,Y,Choppy audio,N,Stepped_improvement,3,Noticeable pixelation during transition
3,Teams,iPhone_13_iOS_16.1,WiFi_to_Cellular,12:19:15,Session_A,0.8,2.1,N,1080p,720p,1080p,30,25,30,N,,N,Quick_recovery,5,Smooth transition with brief quality dip
4,FaceTime,iPhone_13_iOS_16.1,WiFi_to_Cellular,12:25:33,Session_A,5.2,8.7,Y,720p,N/A,720p,30,0,30,Y,Complete audio loss,Y,Hard_restart,2,Call dropped and auto-reconnected
```

## Network Environment Documentation

### Session-Level Network Conditions

Document once per testing session (all apps tested under same conditions):

```csv
Session_ID,Date,Time,Location,WiFi_Signal_Strength,WiFi_Speed_Down_Mbps,WiFi_Speed_Up_Mbps,Cellular_Signal_Strength,Cellular_Speed_Down_Mbps,Cellular_Speed_Up_Mbps,Network_Latency_WiFi_ms,Network_Latency_Cellular_ms,Weather_Conditions,Notes
Session_A,2024-09-15,12:00,Home_Office,-45dBm,50,10,Full_bars,25,8,25,45,Clear,Stable conditions throughout session
```

**Rationale:** Network conditions should be consistent across all app tests in a session. This eliminates redundant per-app network measurement while documenting environmental factors.

## Data Collection Procedure

### Pre-Session Setup (Once per testing session)

- [ ] **Network baseline measurement**
  - Test WiFi speed and latency using Speedtest app
  - Test cellular speed and latency 
  - Record signal strengths for both networks
  - Note environmental factors (location, time, weather)
  - Assign session ID for tracking

- [ ] **Device preparation**
  - Battery level >80% (to last through multiple app tests)
  - Close unnecessary background apps
  - Enable airplane mode, then re-enable WiFi and cellular individually
  - Test both network connections independently

### Per-App Test Procedure

#### Timing Critical Events
1. **T0: Call establishment** - Start call and wait for stable connection (30 seconds)
2. **T1: Handoff trigger** - Execute network switch (disable WiFi or reconnect)
3. **T2: First response** - When app first shows reaction to network change
4. **T3: Service resumption** - When call continues (may be degraded quality)
5. **T4: Full recovery** - When quality returns to pre-handoff level

#### App Response Assessment
- **Immediate reaction**: How quickly does the app detect the change?
- **Degradation pattern**: Gradual quality reduction or immediate freeze?
- **Recovery strategy**: Does the app reconnect or maintain connection?
- **Quality restoration**: How does quality return (gradual, stepped, instant)?

### App-Specific Response Patterns

#### Zoom
- **Typical behavior**: Shows connection warnings, adaptive bitrate adjustment
- **Watch for**: Connection quality indicators, automatic quality changes
- **Screen capture**: Connection statistics if available during transition

#### Microsoft Teams  
- **Typical behavior**: Connection quality meter, may show "poor connection" overlay
- **Watch for**: Quality adaptation speed, reconnection attempts
- **Screen capture**: Call quality dashboard if accessible

#### WhatsApp
- **Typical behavior**: Minimal feedback, relies on visual quality assessment
- **Watch for**: Video pixelation, audio dropouts, call stability
- **Focus on**: User-perceived experience since no technical feedback

#### FaceTime
- **Typical behavior**: Seamless for users, minimal visible feedback during transitions
- **Watch for**: Call drops vs. maintained connection, quality recovery speed
- **Focus on**: Timing of disruptions and recovery

### Data Recording Guidelines

#### App Response Timing
- **Freeze Duration**: Time from network change (T1) to any service resumption (T3)
- **Recovery Time**: Time from network change (T1) to full quality restoration (T4)
- **Detection Latency**: Time from network change (T1) to first app response (T2)

#### Quality Recovery Patterns
- **Gradual_improvement**: Quality slowly returns over 10+ seconds
- **Stepped_improvement**: Quality jumps in discrete steps (360p → 480p → 720p)
- **Quick_recovery**: Quality returns to normal within 3 seconds
- **Hard_restart**: App reconnects rather than maintaining connection

#### Call Continuity Assessment
- **Maintained_connection**: Call continues without reconnection
- **Soft_reconnection**: Brief disconnect with automatic reconnection
- **Hard_reconnection**: User must manually reconnect
- **Call_failure**: Complete call termination

### User Experience Rating Scale

**5 - Seamless**: No noticeable disruption, call continues smoothly  
**4 - Minor**: Brief (<2s) quality dip, call maintains  
**3 - Noticeable**: 2-5s disruption, temporary quality loss but call survives  
**2 - Disruptive**: 5-10s freeze or significant quality impact  
**1 - Poor**: >10s disruption, call drop, or major service failure

## Quality Assurance

### Session Consistency
- **Network stability**: Verify network conditions don't change between app tests
- **Device state**: Ensure device doesn't overheat or run low on battery
- **Environmental control**: Test in same location with consistent interference
- **Timing consistency**: Complete all apps in session within 1-2 hours

### Data Validation
- **Cross-reference timing**: Use screen recordings to verify manual timings
- **Response pattern consistency**: Similar apps should show similar behaviors
- **Missing data handling**: Mark unavailable metrics as "N/A" rather than guessing
- **Outlier investigation**: Investigate unusually good/bad performance

## File Organization

```
docs/baseline-study/results/
├── raw-data/
│   ├── baseline-data.csv              # Main app response data
│   ├── network-sessions.csv           # Session-level network conditions
│   └── screen-recordings/             # Video recordings by trial
│       ├── Session_A_Zoom.mp4
│       ├── Session_A_WhatsApp.mp4
│       └── ...
├── analysis/
│   ├── app-comparison.csv             # Cross-app response analysis
│   └── recovery-patterns.csv          # Quality recovery categorization
└── notes/
    ├── session-observations.md        # Per-session qualitative notes
    └── app-behavior-patterns.md       # Observed app-specific behaviors
```

## Analysis Focus Areas

### App Response Comparison
- **Detection speed**: How quickly each app responds to network changes
- **Recovery strategies**: Maintained connection vs. reconnection approaches
- **Quality adaptation**: Bitrate adjustment patterns during transitions
- **User feedback**: What information apps provide during handovers

### User Experience Patterns
- **Perceived disruption**: Which apps feel smoothest during transitions
- **Recovery predictability**: Consistent vs. variable response patterns
- **Error handling**: How apps communicate issues to users

### Success Metrics for QuicVid Comparison
- **Freeze duration reduction**: Target <1s vs. current app performance
- **Call survival improvement**: Maintain connection vs. reconnection patterns
- **Quality recovery speed**: Faster restoration compared to baseline apps
- **User preference**: Measurable improvement in experience ratings
