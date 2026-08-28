import 'package:flutter/material.dart';

import '../services/auth_service.dart';
import '../services/localization_service.dart';

/// 登录弹窗，返回是否登录成功。预订流程与我的页共用。
Future<bool> showLoginDialog(BuildContext context) async {
  final loc = LocalizationService.instance;
  final email = TextEditingController();
  final password = TextEditingController();
  var ok = false;
  await showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(loc.getString('profile.login')),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: email,
            keyboardType: TextInputType.emailAddress,
            decoration: const InputDecoration(labelText: 'Email'),
          ),
          TextField(
            controller: password,
            obscureText: true,
            decoration: const InputDecoration(labelText: 'Password'),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(loc.getString('common.cancel')),
        ),
        FilledButton(
          onPressed: () async {
            try {
              await AuthService.instance.login(email.text, password.text);
              ok = true;
              if (context.mounted) Navigator.pop(context);
            } on Exception {
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text(loc.getString('common.error'))),
                );
              }
            }
          },
          child: Text(loc.getString('profile.login')),
        ),
      ],
    ),
  );
  return ok;
}
