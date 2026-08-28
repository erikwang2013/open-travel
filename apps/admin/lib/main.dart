import 'package:flutter/material.dart';

void main() {
  runApp(const TravelAdminApp());
}

/// Open Travel 管理端（Flutter Web）：后台管理入口骨架。
/// 后续在此扩展登录、目的地/订单/用户管理等管理页面。
class TravelAdminApp extends StatelessWidget {
  const TravelAdminApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Travel Admin',
      theme: ThemeData(colorSchemeSeed: Colors.blue, useMaterial3: true),
      home: const HomePage(),
    );
  }
}

class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Open Travel 管理端')),
      body: const Center(child: Text('管理后台开发中 — 待扩展')),
    );
  }
}
