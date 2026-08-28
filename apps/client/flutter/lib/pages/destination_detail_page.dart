import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/content_service.dart';
import '../services/localization_service.dart';
import 'lines_page.dart';

class DestinationDetailPage extends StatefulWidget {
  const DestinationDetailPage({super.key, required this.destination});

  final Destination destination;

  @override
  State<DestinationDetailPage> createState() => _DestinationDetailPageState();
}

class _DestinationDetailPageState extends State<DestinationDetailPage> {
  late Future<List<Attraction>> _attractions;

  @override
  void initState() {
    super.initState();
    _attractions = ContentService.instance
        .fetchAttractions(destinationId: widget.destination.id);
  }

  void _reload() {
    setState(() {
      _attractions = ContentService.instance
          .fetchAttractions(destinationId: widget.destination.id);
    });
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final d = widget.destination;
    return Scaffold(
      appBar: AppBar(title: Text(d.name)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: AspectRatio(
              aspectRatio: 16 / 9,
              child: d.coverUrl.isNotEmpty
                  ? Image.network(
                      d.coverUrl,
                      fit: BoxFit.cover,
                      errorBuilder: (_, _, _) => const _ImagePlaceholder(),
                    )
                  : const _ImagePlaceholder(),
            ),
          ),
          if (d.description.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(d.description),
          ],
          const SizedBox(height: 16),
          FilledButton.icon(
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute(
                builder: (_) => LinesPage(destinationId: d.id, destinationName: d.name),
              ),
            ),
            icon: const Icon(Icons.route),
            label: Text(loc.getString('lines.viewLines')),
          ),
          const SizedBox(height: 24),
          Text(
            loc.getString('detail.attractions'),
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 12),
          FutureBuilder<List<Attraction>>(
            future: _attractions,
            builder: (context, snapshot) {
              if (snapshot.hasError) {
                return Column(
                  children: [
                    Text(loc.getString('common.loadFailed')),
                    TextButton(
                      onPressed: _reload,
                      child: Text(loc.getString('common.retry')),
                    ),
                  ],
                );
              }
              if (!snapshot.hasData) {
                return const Padding(
                  padding: EdgeInsets.all(24),
                  child: Center(child: CircularProgressIndicator()),
                );
              }
              final list = snapshot.data!;
              if (list.isEmpty) return Text(loc.getString('common.noData'));
              return Column(
                children: [for (final a in list) _AttractionCard(attraction: a)],
              );
            },
          ),
        ],
      ),
    );
  }
}

class _AttractionCard extends StatelessWidget {
  const _AttractionCard({required this.attraction});

  final Attraction attraction;

  @override
  Widget build(BuildContext context) {
    final a = attraction;
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          ClipRRect(
            borderRadius: const BorderRadius.vertical(top: Radius.circular(12)),
            child: AspectRatio(
              aspectRatio: 16 / 9,
              child: a.coverUrl.isNotEmpty
                  ? Image.network(
                      a.coverUrl,
                      fit: BoxFit.cover,
                      errorBuilder: (_, _, _) => const _ImagePlaceholder(),
                    )
                  : const _ImagePlaceholder(),
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        a.name,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                    ),
                    if (a.rating > 0) StarRating(rating: a.rating),
                  ],
                ),
                if (a.description.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Text(a.description),
                ],
                const SizedBox(height: 8),
                Wrap(
                  spacing: 16,
                  runSpacing: 4,
                  children: [
                    if (a.priceCents > 0)
                      _InfoChip(icon: Icons.payments, text: formatYuan(a.priceCents)),
                    if (a.openHours.isNotEmpty)
                      _InfoChip(icon: Icons.schedule, text: a.openHours),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoChip extends StatelessWidget {
  const _InfoChip({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 16, color: scheme.primary),
        const SizedBox(width: 4),
        Text(text, style: Theme.of(context).textTheme.bodyMedium),
      ],
    );
  }
}

/// 五星展示：按 rating 取整显示实心/半星/空心。
class StarRating extends StatelessWidget {
  const StarRating({super.key, required this.rating});

  final double rating;

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.primary;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var i = 1; i <= 5; i++)
          Icon(
            rating >= i
                ? Icons.star
                : rating >= i - 0.5
                    ? Icons.star_half
                    : Icons.star_border,
            size: 16,
            color: color,
          ),
        const SizedBox(width: 4),
        Text(rating.toStringAsFixed(1)),
      ],
    );
  }
}

class _ImagePlaceholder extends StatelessWidget {
  const _ImagePlaceholder();

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      child: Icon(
        Icons.landscape,
        size: 48,
        color: Theme.of(context).colorScheme.outline,
      ),
    );
  }
}
