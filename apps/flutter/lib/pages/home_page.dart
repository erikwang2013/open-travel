import 'package:flutter/material.dart';

import '../services/api_client.dart';
import '../services/localization_service.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  static const List<String> _destinations = [
    'Paris',
    'Tokyo',
    'Bali',
    'Reykjavik',
    'Cairo',
    'Lisbon',
  ];

  late final Future<List<String>> _dates;

  @override
  void initState() {
    super.initState();
    _dates = _fetchDates();
  }

  Future<List<String>> _fetchDates() async {
    try {
      final res = await ApiClient.instance.dio
          .get<List<dynamic>>('/api/v1/booking/dates');
      return res.data?.whereType<String>().toList() ?? const [];
    } on Exception {
      // ponytail: hardcoded fallback, replace once booking API is live
      return const ['2026-09-01', '2026-09-15', '2026-10-01'];
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        TextField(
          decoration: InputDecoration(
            hintText: loc.getString('search.placeholder'),
            prefixIcon: const Icon(Icons.search),
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
          ),
        ),
        const SizedBox(height: 24),
        Text(
          loc.getString('home.heroTitle'),
          style: Theme.of(context).textTheme.headlineSmall,
        ),
        const SizedBox(height: 16),
        Text(
          loc.getString('home.popularDestinations'),
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 12),
        GridView.count(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          crossAxisCount: 2,
          mainAxisSpacing: 12,
          crossAxisSpacing: 12,
          childAspectRatio: 1.6,
          children: [
            for (final name in _destinations)
              Card(
                child: Center(
                  child: Text(
                    name,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ),
          ],
        ),
        const SizedBox(height: 16),
        FutureBuilder<List<String>>(
          future: _dates,
          builder: (context, snapshot) {
            if (!snapshot.hasData) {
              return const Padding(
                padding: EdgeInsets.all(24),
                child: Center(child: CircularProgressIndicator()),
              );
            }
            final dates = snapshot.data!;
            if (dates.isEmpty) {
              return Text(loc.getString('common.noData'));
            }
            return Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [for (final d in dates) Chip(label: Text(d))],
            );
          },
        ),
      ],
    );
  }
}
