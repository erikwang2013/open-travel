import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart';

import '../services/auth_service.dart';
import '../services/localization_service.dart';

class ProfilePage extends StatelessWidget {
  const ProfilePage({super.key});

  static const Map<String, String> _languageNames = {
    'en': 'English',
    'zh': '中文',
    'ja': '日本語',
    'ko': '한국어',
    'ru': 'Русский',
    'de': 'Deutsch',
    'fr': 'Français',
    'es': 'Español',
    'pt': 'Português',
    'hi': 'हिन्दी',
    'ar': 'العربية',
    'bn': 'বাংলা',
    'id': 'Bahasa Indonesia',
  };

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final auth = AuthService.instance;
    return ListenableBuilder(
      listenable: Listenable.merge([loc, auth]),
      builder: (context, _) {
        final profile = auth.profile;
        return ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Text(
              loc.getString('profile.title'),
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            Center(
              child: CircleAvatar(
                radius: 40,
                child: Icon(Icons.person, size: 40),
              ),
            ),
            const SizedBox(height: 8),
            Center(
              child: Text(
                profile?.nickname.isNotEmpty == true
                    ? profile!.nickname
                    : profile?.email ?? loc.getString('profile.notLoggedIn'),
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            const SizedBox(height: 16),
            if (auth.isLoggedIn)
              OutlinedButton.icon(
                onPressed: () => _showEditDialog(context),
                icon: const Icon(Icons.edit_outlined),
                label: Text(loc.getString('profile.edit')),
              )
            else
              FilledButton.icon(
                onPressed: () => _showLoginDialog(context),
                icon: const Icon(Icons.login),
                label: Text(loc.getString('profile.login')),
              ),
            const SizedBox(height: 24),
            Text(
              loc.getString('profile.lang'),
              style: Theme.of(context).textTheme.titleLarge,
            ),
            DropdownButton<Locale>(
              value: loc.locale,
              isExpanded: true,
              items: [
                for (final l in LocalizationService.supportedLocales)
                  DropdownMenuItem(
                    value: l,
                    child: Text(_languageNames[l.languageCode] ?? l.languageCode),
                  ),
              ],
              onChanged: (l) {
                if (l != null) loc.loadLanguage(l);
              },
            ),
            const SizedBox(height: 24),
            const _AddressBookSection(),
            const SizedBox(height: 24),
            Text(
              loc.getString('profile.contactSupport'),
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 8),
            Card(
              child: ListTile(
                leading: const Icon(Icons.support_agent),
                title: Text(loc.getString('profile.contactSupport')),
                trailing: const Icon(Icons.chevron_right),
                onTap: () => _showSupportDialog(context),
              ),
            ),
          ],
        );
      },
    );
  }

  Future<void> _showLoginDialog(BuildContext context) async {
    final loc = LocalizationService.instance;
    final email = TextEditingController();
    final password = TextEditingController();
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
  }

  Future<void> _showEditDialog(BuildContext context) async {
    final loc = LocalizationService.instance;
    final auth = AuthService.instance;
    final nickname = TextEditingController(text: auth.profile?.nickname ?? '');
    var lang = loc.locale;
    await showDialog<void>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          title: Text(loc.getString('profile.edit')),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: nickname,
                decoration: InputDecoration(labelText: loc.getString('profile.nickname')),
              ),
              const SizedBox(height: 16),
              DropdownButton<Locale>(
                value: lang,
                isExpanded: true,
                items: [
                  for (final l in LocalizationService.supportedLocales)
                    DropdownMenuItem(
                      value: l,
                      child: Text(_languageNames[l.languageCode] ?? l.languageCode),
                    ),
                ],
                onChanged: (l) => setState(() => lang = l!),
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
                  await auth.updateProfile(
                    nickname: nickname.text.trim(),
                    lang: lang.languageCode,
                  );
                  await loc.loadLanguage(lang);
                  if (context.mounted) Navigator.pop(context);
                } on Exception {
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(content: Text(loc.getString('common.error'))),
                    );
                  }
                }
              },
              child: Text(loc.getString('profile.save')),
            ),
          ],
        ),
      ),
    );
  }

  void _showSupportDialog(BuildContext context) {
    final loc = LocalizationService.instance;
    showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(loc.getString('profile.contactSupport')),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: const Icon(Icons.phone),
              title: const Text('+86 400-000-0000'),
              onTap: () => launchUrl(Uri.parse('tel:+864000000000')),
            ),
            ListTile(
              leading: const Icon(Icons.mail_outline),
              title: const Text('support@erik.xyz'),
              onTap: () => launchUrl(Uri.parse('mailto:support@erik.xyz')),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: Text(loc.getString('common.cancel')),
          ),
        ],
      ),
    );
  }
}

class _AddressBookSection extends StatefulWidget {
  const _AddressBookSection();

  @override
  State<_AddressBookSection> createState() => _AddressBookSectionState();
}

class _AddressBookSectionState extends State<_AddressBookSection> {
  static const _prefsKey = 'address_book';

  List<Map<String, String>> _addresses = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final prefs = await SharedPreferences.getInstance();
    final raw = prefs.getString(_prefsKey);
    if (raw == null) return;
    final decoded = jsonDecode(raw);
    if (decoded is List) {
      setState(() {
        _addresses = [
          for (final item in decoded)
            if (item is Map)
              {
                'label': (item['label'] as String?) ?? '',
                'address': (item['address'] as String?) ?? '',
              },
        ];
      });
    }
  }

  Future<void> _save() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefsKey, jsonEncode(_addresses));
  }

  Future<void> _add() async {
    final loc = LocalizationService.instance;
    final label = TextEditingController();
    final address = TextEditingController();
    final ok = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(loc.getString('profile.addAddress')),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: label,
              decoration: InputDecoration(labelText: loc.getString('profile.nickname')),
            ),
            TextField(
              controller: address,
              decoration: InputDecoration(labelText: loc.getString('profile.address')),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(loc.getString('common.cancel')),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(loc.getString('profile.save')),
          ),
        ],
      ),
    );
    if (ok == true && address.text.trim().isNotEmpty) {
      setState(() {
        _addresses.add({'label': label.text.trim(), 'address': address.text.trim()});
      });
      await _save();
    }
  }

  Future<void> _remove(int index) async {
    setState(() => _addresses.removeAt(index));
    await _save();
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              loc.getString('profile.addressBook'),
              style: Theme.of(context).textTheme.titleLarge,
            ),
            IconButton(
              icon: const Icon(Icons.add_circle_outline),
              tooltip: loc.getString('profile.addAddress'),
              onPressed: _add,
            ),
          ],
        ),
        if (_addresses.isEmpty)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(loc.getString('common.noData')),
          )
        else
          for (var i = 0; i < _addresses.length; i++)
            Card(
              child: ListTile(
                leading: const Icon(Icons.location_on_outlined),
                title: Text(_addresses[i]['label']!.isEmpty
                    ? _addresses[i]['address']!
                    : _addresses[i]['label']!),
                subtitle: Text(_addresses[i]['address']!),
                trailing: IconButton(
                  icon: const Icon(Icons.delete_outline),
                  tooltip: loc.getString('profile.delete'),
                  onPressed: () => _remove(i),
                ),
              ),
            ),
      ],
    );
  }
}
