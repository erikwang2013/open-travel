import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'pages/home_shell.dart';
import 'services/auth_service.dart';
import 'services/localization_service.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await LocalizationService.instance.init();
  await AuthService.instance.init();
  runApp(const OpenTravelApp());
}

class OpenTravelApp extends StatelessWidget {
  const OpenTravelApp({super.key});

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return ListenableBuilder(
      listenable: loc,
      builder: (context, _) => MaterialApp(
        title: loc.getString('appTitle'),
        theme: ThemeData(colorSchemeSeed: Colors.teal, useMaterial3: true),
        locale: loc.locale,
        supportedLocales: LocalizationService.supportedLocales,
        localizationsDelegates: const [
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        home: const HomeShell(),
      ),
    );
  }
}
