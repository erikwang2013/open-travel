import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/content_service.dart';
import '../services/localization_service.dart';
import 'destination_detail_page.dart';
import 'flight_search_page.dart';
import 'hotel_search_page.dart';
import 'search_page.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  static const List<String> _fallbackDestinations = [
    'Paris',
    'Tokyo',
    'Bali',
    'Reykjavik',
    'Cairo',
    'Lisbon',
  ];

  late Future<List<Destination>> _destinations;
  String _loadedLang = '';

  @override
  void initState() {
    super.initState();
    _destinations = _fetchDestinations();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final lang = LocalizationService.instance.locale.languageCode;
    if (lang != _loadedLang) {
      _loadedLang = lang;
      setState(() => _destinations = _fetchDestinations());
    }
  }

  Future<List<Destination>> _fetchDestinations() async {
    try {
      final list = await ContentService.instance.fetchDestinations();
      if (list.isNotEmpty) return list;
    } on Exception {
      // ponytail: API 失败兜底硬编码，避免空屏
    }
    return [
      for (var i = 0; i < _fallbackDestinations.length; i++)
        Destination(id: i + 1, name: _fallbackDestinations[i]),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        TextField(
          readOnly: true,
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute(builder: (_) => const SearchPage()),
          ),
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
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: _QuickEntryCard(
                icon: Icons.flight_takeoff,
                label: loc.getString('flight.title'),
                onTap: () => Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const FlightSearchPage()),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: _QuickEntryCard(
                icon: Icons.hotel,
                label: loc.getString('hotel.title'),
                onTap: () => Navigator.of(context).push(
                  MaterialPageRoute(builder: (_) => const HotelSearchPage()),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Text(
          loc.getString('home.popularDestinations'),
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 12),
        FutureBuilder<List<Destination>>(
          future: _destinations,
          builder: (context, snapshot) {
            if (!snapshot.hasData) {
              return const Padding(
                padding: EdgeInsets.all(24),
                child: Center(child: CircularProgressIndicator()),
              );
            }
            return GridView.count(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              crossAxisCount: 2,
              mainAxisSpacing: 12,
              crossAxisSpacing: 12,
              childAspectRatio: 1.6,
              children: [
                for (final d in snapshot.data!)
                  _DestinationCard(destination: d),
              ],
            );
          },
        ),
      ],
    );
  }
}

class _DestinationCard extends StatelessWidget {
  const _DestinationCard({required this.destination});

  final Destination destination;

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => DestinationDetailPage(destination: destination),
          ),
        ),
        child: Stack(
          fit: StackFit.expand,
          children: [
            if (destination.coverUrl.isNotEmpty)
              Image.network(
                destination.coverUrl,
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) => const _CardPlaceholder(),
              )
            else
              const _CardPlaceholder(),
            Align(
              alignment: Alignment.bottomLeft,
              child: Container(
                width: double.infinity,
                padding: const EdgeInsets.all(8),
                color: Colors.black45,
                child: Text(
                  destination.name,
                  style: Theme.of(context)
                      .textTheme
                      .titleMedium
                      ?.copyWith(color: Colors.white),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _QuickEntryCard extends StatelessWidget {
  const _QuickEntryCard({required this.icon, required this.label, required this.onTap});

  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Card(
      margin: EdgeInsets.zero,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 16),
          child: Column(
            children: [
              Icon(icon, size: 32, color: scheme.primary),
              const SizedBox(height: 8),
              Text(label, style: Theme.of(context).textTheme.titleSmall),
            ],
          ),
        ),
      ),
    );
  }
}

class _CardPlaceholder extends StatelessWidget {
  const _CardPlaceholder();

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      child: Icon(
        Icons.landscape,
        size: 40,
        color: Theme.of(context).colorScheme.outline,
      ),
    );
  }
}
