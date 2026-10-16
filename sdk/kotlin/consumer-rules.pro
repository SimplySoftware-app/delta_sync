# UniFFI bindings are reflected on by JNA.
-keep class com.sun.jna.** { *; }
-keep class uniffi.delta_sync_core.** { *; }
-dontwarn java.awt.*
