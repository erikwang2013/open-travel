import 'package:flutter/material.dart';

import 'api.dart';
import 'pages/home_page.dart';
import 'pages/login_page.dart';

void main() {
  runApp(const TravelAdminApp());
}

class TravelAdminApp extends StatelessWidget {
  const TravelAdminApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Travel Admin',
      theme: ThemeData(colorSchemeSeed: Colors.blue, useMaterial3: true),
      home: ValueListenableBuilder<String?>(
        valueListenable: AuthService.instance.token,
        builder: (context, token, _) =>
            token == null ? const LoginPage() : const HomePage(),
      ),
    );
  }
}
