import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import 'line_detail_page.dart';

/// 线路列表：按目的地进入，展示封面/标题/天数/价格/成团人数。
class LinesPage extends StatefulWidget {
  const LinesPage({super.key, this.destinationId, this.destinationName = ''});

  final int? destinationId;
  final String destinationName;

  @override
  State<LinesPage> createState() => _LinesPageState();
}

class _LinesPageState extends State<LinesPage> {
  late Future<List<Line>> _future;

  @override
  void initState() {
    super.initState();
    _future = OrderService.instance.fetchLines(destinationId: widget.destinationId);
  }

  void _reload() {
    setState(() {
      _future = OrderService.instance.fetchLines(destinationId: widget.destinationId);
    });
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final title = widget.destinationName.isNotEmpty
        ? '${widget.destinationName} · ${loc.getString('lines.title')}'
        : loc.getString('lines.title');
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: FutureBuilder<List<Line>>(
        future: _future,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(loc.getString('common.loadFailed')),
                  TextButton(onPressed: _reload, child: Text(loc.getString('common.retry'))),
                ],
              ),
            );
          }
          if (!snapshot.hasData) {
            return const Center(child: CircularProgressIndicator());
          }
          final list = snapshot.data!;
          if (list.isEmpty) {
            return Center(child: Text(loc.getString('common.noData')));
          }
          return ListView.builder(
            padding: const EdgeInsets.all(16),
            itemCount: list.length,
            itemBuilder: (context, i) => _LineCard(line: list[i]),
          );
        },
      ),
    );
  }
}

class _LineCard extends StatelessWidget {
  const _LineCard({required this.line});

  final Line line;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => Navigator.of(context).push(
          MaterialPageRoute(builder: (_) => LineDetailPage(line: line)),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (line.coverUrl.isNotEmpty)
              AspectRatio(
                aspectRatio: 16 / 9,
                child: Image.network(
                  line.coverUrl,
                  fit: BoxFit.cover,
                  errorBuilder: (_, _, _) => _placeholder(context),
                ),
              )
            else
              AspectRatio(aspectRatio: 16 / 9, child: _placeholder(context)),
            Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(line.title, style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 16,
                    runSpacing: 4,
                    children: [
                      _InfoChip(icon: Icons.calendar_month, text: '${line.days}${loc.getString('lines.dayUnit')}'),
                      if (line.maxPax > 0)
                        _InfoChip(icon: Icons.groups, text: '${line.maxPax}${loc.getString('lines.minGroup')}'),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    formatYuan(line.priceCents),
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                          color: Theme.of(context).colorScheme.primary,
                        ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _placeholder(BuildContext context) => Container(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        child: Icon(Icons.route, size: 48, color: Theme.of(context).colorScheme.outline),
      );
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
