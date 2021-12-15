# Real-time Lip Sync GD
A Rust port of [uLipSync](https://github.com/hecomi/uLipSync) connected to Godot via [godot-rust](https://github.com/godot-rust/godot-rust). In theory, should work with any game engine with a C api.

## Porting notes
- uLipSync
    - Runtime
        - Core
            - [x] Algorithm.cs
            - [x] Common.cs
            - [x] LipSyncJob.cs
            - [ ] ~~MicUtil.cs~~
                - Won't do, should be done from GDScript
            - [x] Profile.cs
        - [ ] uLipSync.cs
        - [ ] uLipSyncAudioSource.cs
            - I don't think I need this?
        - [ ] uLipSyncBlendShape.cs
        - [ ] ~~uLipSyncMicrophone.cs~~
            - Won't do, should be done from GDScript
