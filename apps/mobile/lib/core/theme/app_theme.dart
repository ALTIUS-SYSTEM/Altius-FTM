import 'package:flutter/material.dart';

class AppTheme {
  static const teal = Color(0xFF00677E);
  static const cyan = Color(0xFF0FB4DA);
  static const amber = Color(0xFFE3942C);
  static ThemeData get light => ThemeData(
    useMaterial3: true,
    scaffoldBackgroundColor: const Color(0xFFF5FAFD),
    colorScheme: ColorScheme.fromSeed(seedColor: teal, brightness: Brightness.light).copyWith(primary: teal, primaryContainer: cyan, onPrimaryContainer: const Color(0xFF004151), secondary: const Color(0xFF3A6473), surface: const Color(0xFFF5FAFD)),
    appBarTheme: const AppBarTheme(backgroundColor: Color(0xFFF5FAFD), surfaceTintColor: Colors.transparent, centerTitle: false),
    cardTheme: CardThemeData(elevation: 0, color: Colors.white, margin: const EdgeInsets.only(bottom: 16), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28), side: const BorderSide(color: Color(0xFFE0E9ED)))),
    filledButtonTheme: FilledButtonThemeData(style: FilledButton.styleFrom(minimumSize: const Size(48, 54), backgroundColor: cyan, foregroundColor: const Color(0xFF004151), textStyle: const TextStyle(fontWeight: FontWeight.w700, fontSize: 15), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)))),
    outlinedButtonTheme: OutlinedButtonThemeData(style: OutlinedButton.styleFrom(minimumSize: const Size(48, 50), shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)))),
    inputDecorationTheme: InputDecorationTheme(filled: true, fillColor: Colors.white, contentPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 18), border: OutlineInputBorder(borderRadius: BorderRadius.circular(16), borderSide: const BorderSide(color: Color(0xFFBCC8CE))), enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(16), borderSide: const BorderSide(color: Color(0xFFBCC8CE)))),
    navigationBarTheme: NavigationBarThemeData(backgroundColor: Colors.white, indicatorColor: const Color(0xFFD7F3FA), labelTextStyle: WidgetStateProperty.resolveWith((states) => TextStyle(fontSize: 11, fontWeight: states.contains(WidgetState.selected) ? FontWeight.w700 : FontWeight.w500))),
    textTheme: const TextTheme(headlineLarge: TextStyle(fontSize: 32, fontWeight: FontWeight.w700, letterSpacing: -1.1), headlineSmall: TextStyle(fontSize: 24, fontWeight: FontWeight.w700, letterSpacing: -.6), titleLarge: TextStyle(fontSize: 20, fontWeight: FontWeight.w700), titleMedium: TextStyle(fontSize: 16, fontWeight: FontWeight.w700), bodyMedium: TextStyle(fontSize: 14, height: 1.5, color: Color(0xFF3D494D))),
  );
}
