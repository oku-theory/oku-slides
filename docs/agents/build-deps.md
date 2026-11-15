# Build dependencies

Read before adding a WinRT API.

The workspace `windows` 0.62 entry (default features off, for the stt Windows voice) lists exactly `Media_SpeechSynthesis`, `Media_Playback`, `Media_Core`, `Storage_Streams`, `Foundation`, `Foundation_Collections`. A new WinRT API needs its feature added there, and only a Windows build compiles it.
