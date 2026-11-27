# Keep ML Kit text recognition optional language modules (not included).
-dontwarn com.google.mlkit.vision.text.chinese.**
-dontwarn com.google.mlkit.vision.text.devanagari.**
-dontwarn com.google.mlkit.vision.text.japanese.**
-dontwarn com.google.mlkit.vision.text.korean.**

# Keep google_mlkit_text_recognition plugin.
-keep class com.google_mlkit_text_recognition.** { *; }
