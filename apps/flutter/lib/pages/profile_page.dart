import 'package:flutter/material.dart';

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
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          loc.getString('profile.title'),
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 16),
        const CircleAvatar(radius: 40, child: Icon(Icons.person, size: 40)),
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
        Text(loc.getString('common.welcome')),
      ],
    );
  }
}
