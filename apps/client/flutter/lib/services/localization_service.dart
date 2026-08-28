import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

class LocalizationService extends ChangeNotifier {
  LocalizationService._();

  static final LocalizationService instance = LocalizationService._();

  static const List<Locale> supportedLocales = [
    Locale('en'),
    Locale('zh'),
    Locale('ja'),
    Locale('ko'),
    Locale('ru'),
    Locale('de'),
    Locale('fr'),
    Locale('es'),
    Locale('pt'),
    Locale('hi'),
    Locale('ar'),
    Locale('bn'),
    Locale('id'),
  ];

  static const Set<String> _rtlLanguages = {'ar'};

  Locale _locale = const Locale('en');
  Map<String, dynamic>? _strings;
  Map<String, dynamic>? _en;

  Locale get locale => _locale;
  bool get isRtl => _rtlLanguages.contains(_locale.languageCode);

  /// Uses the system locale when supported, otherwise falls back to en.
  Future<void> init() async {
    final system = WidgetsBinding.instance.platformDispatcher.locale;
    final supported = supportedLocales.any(
      (l) => l.languageCode == system.languageCode,
    );
    await loadLanguage(
      supported ? Locale(system.languageCode) : const Locale('en'),
    );
  }

  Future<void> loadLanguage(Locale locale) async {
    _strings = await _load('assets/i18n/${locale.languageCode}.arb');
    if (locale.languageCode != 'en') {
      _en ??= await _load('assets/i18n/en.arb');
    }
    _locale = locale;
    notifyListeners();
  }

  /// Current language first, then en, then the key itself as last resort.
  String getString(String key) {
    final value = _strings?[key] ?? _en?[key];
    return value is String ? value : key;
  }

  Future<Map<String, dynamic>> _load(String path) async =>
      jsonDecode(await rootBundle.loadString(path)) as Map<String, dynamic>;
}
