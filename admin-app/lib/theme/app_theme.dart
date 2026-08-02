import 'package:flutter/material.dart';

abstract final class AppTheme {
  static const ink = Color(0xFF1F2328);
  static const muted = Color(0xFF656D76);
  static const line = Color(0xFFD0D7DE);
  static const canvas = Color(0xFFF6F8FA);
  static const surface = Colors.white;

  static ThemeData get light {
    final scheme =
        ColorScheme.fromSeed(
          seedColor: ink,
          brightness: Brightness.light,
          surface: surface,
        ).copyWith(
          primary: ink,
          onPrimary: Colors.white,
          outline: line,
          surfaceContainer: canvas,
          error: const Color(0xFFCF222E),
        );
    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: canvas,
      hoverColor: Colors.transparent,
      textTheme: const TextTheme(
        headlineSmall: TextStyle(fontSize: 22, fontWeight: FontWeight.w700),
        titleLarge: TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
        bodyMedium: TextStyle(fontSize: 14, color: ink),
      ),
      appBarTheme: const AppBarTheme(
        centerTitle: false,
        elevation: 0,
        backgroundColor: canvas,
        foregroundColor: ink,
        titleTextStyle: TextStyle(
          color: ink,
          fontSize: 20,
          fontWeight: FontWeight.w700,
        ),
      ),
      navigationBarTheme: NavigationBarThemeData(
        height: 72,
        backgroundColor: surface,
        indicatorColor: ink,
        labelTextStyle: WidgetStateProperty.resolveWith(
          (states) => TextStyle(
            color: ink,
            fontSize: 12,
            fontWeight: states.contains(WidgetState.selected)
                ? FontWeight.w700
                : FontWeight.w500,
          ),
        ),
        iconTheme: WidgetStateProperty.resolveWith(
          (states) => IconThemeData(
            color: states.contains(WidgetState.selected) ? Colors.white : muted,
          ),
        ),
      ),
      segmentedButtonTheme: SegmentedButtonThemeData(
        style: ButtonStyle(
          minimumSize: const WidgetStatePropertyAll(Size(88, 44)),
          overlayColor: WidgetStateProperty.resolveWith(_buttonOverlay),
          shape: WidgetStatePropertyAll(
            RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
          ),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: surface,
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 14,
          vertical: 14,
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style:
            FilledButton.styleFrom(
              minimumSize: const Size(44, 48),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
            ).copyWith(
              overlayColor: WidgetStateProperty.resolveWith(_buttonOverlay),
            ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: ButtonStyle(
          minimumSize: const WidgetStatePropertyAll(Size(44, 44)),
          overlayColor: WidgetStateProperty.resolveWith(_buttonOverlay),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: ButtonStyle(
          minimumSize: const WidgetStatePropertyAll(Size(44, 44)),
          overlayColor: WidgetStateProperty.resolveWith(_buttonOverlay),
        ),
      ),
      iconButtonTheme: IconButtonThemeData(
        style: ButtonStyle(
          minimumSize: const WidgetStatePropertyAll(Size(44, 44)),
          overlayColor: WidgetStateProperty.resolveWith(_buttonOverlay),
        ),
      ),
      switchTheme: SwitchThemeData(
        overlayColor: WidgetStateProperty.resolveWith(_buttonOverlay),
      ),
    );
  }

  static Color? _buttonOverlay(Set<WidgetState> states) {
    if (states.contains(WidgetState.hovered)) return Colors.transparent;
    if (states.contains(WidgetState.pressed)) {
      return ink.withValues(alpha: 0.12);
    }
    if (states.contains(WidgetState.focused)) {
      return ink.withValues(alpha: 0.10);
    }
    return null;
  }
}
