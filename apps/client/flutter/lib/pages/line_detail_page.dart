import 'package:dio/dio.dart';
import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/auth_service.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import '../widgets/login_dialog.dart';
import 'order_detail_page.dart';

/// 线路详情：行程安排逐日展示 + 出发日期选择 + 提交订单（P3-12 预订流程）。
class LineDetailPage extends StatefulWidget {
  const LineDetailPage({super.key, required this.line});

  final Line line;

  @override
  State<LineDetailPage> createState() => _LineDetailPageState();
}

class _LineDetailPageState extends State<LineDetailPage> {
  Line? _detail;
  List<LineDate> _dates = [];
  bool _loading = true;
  bool _submitting = false;
  LineDate? _selected;
  int _quantity = 1;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _loading = true);
    try {
      final detail = await OrderService.instance.fetchLine(widget.line.id);
      final dates = await OrderService.instance.fetchLineDates(widget.line.id);
      if (!mounted) return;
      setState(() {
        _detail = detail;
        _dates = dates;
        _loading = false;
      });
    } on Exception {
      if (!mounted) return;
      setState(() => _loading = false);
    }
  }

  Future<void> _submit() async {
    final loc = LocalizationService.instance;
    if (!AuthService.instance.isLoggedIn) {
      final ok = await showLoginDialog(context);
      if (!ok || !mounted) return;
    }
    setState(() => _submitting = true);
    try {
      final order = await OrderService.instance.createOrder(
        productId: widget.line.id,
        lineDateId: _selected!.id > 0 ? _selected!.id : _selected!.date,
        quantity: _quantity,
      );
      if (!mounted) return;
      setState(() => _submitting = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(loc.getString('order.createSuccess'))),
      );
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => OrderDetailPage(orderId: order.id)),
      );
    } on DioException catch (e) {
      if (!mounted) return;
      setState(() => _submitting = false);
      final msg = e.response?.statusCode == 409
          ? loc.getString('order.stockNotEnough')
          : loc.getString('common.error');
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
    } on Exception {
      if (!mounted) return;
      setState(() => _submitting = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(loc.getString('common.error'))),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final line = _detail ?? widget.line;
    return Scaffold(
      appBar: AppBar(title: Text(line.title)),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                if (line.coverUrl.isNotEmpty)
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: AspectRatio(
                      aspectRatio: 16 / 9,
                      child: Image.network(line.coverUrl, fit: BoxFit.cover,
                          errorBuilder: (_, _, _) => const SizedBox.shrink()),
                    ),
                  ),
                const SizedBox(height: 12),
                Text(line.title, style: Theme.of(context).textTheme.headlineSmall),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 16,
                  runSpacing: 4,
                  children: [
                    _InfoChip(icon: Icons.calendar_month, text: '${line.days}${loc.getString('lines.dayUnit')}'),
                    if (line.maxPax > 0)
                      _InfoChip(icon: Icons.groups, text: '${line.maxPax}${loc.getString('lines.minGroup')}'),
                    _InfoChip(icon: Icons.payments, text: formatYuan(line.priceCents)),
                  ],
                ),
                if (line.itinerary.isNotEmpty) ...[
                  const SizedBox(height: 24),
                  Text(loc.getString('lines.itinerary'), style: Theme.of(context).textTheme.titleLarge),
                  const SizedBox(height: 8),
                  for (final day in line.itinerary) _ItineraryCard(day: day),
                ],
                const SizedBox(height: 24),
                Text(loc.getString('lines.selectDate'), style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                if (_dates.isEmpty)
                  Text(loc.getString('lines.noDates'))
                else
                  for (final d in _dates) _DateRow(date: d, selected: _selected == d, onTap: () => setState(() => _selected = d)),
                const SizedBox(height: 16),
                Row(
                  children: [
                    Text(loc.getString('common.quantity')),
                    const Spacer(),
                    IconButton(
                      onPressed: _quantity > 1 ? () => setState(() => _quantity--) : null,
                      icon: const Icon(Icons.remove_circle_outline),
                    ),
                    Text('$_quantity', style: Theme.of(context).textTheme.titleMedium),
                    IconButton(
                      onPressed: _quantity < 9 ? () => setState(() => _quantity++) : null,
                      icon: const Icon(Icons.add_circle_outline),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: _selected == null || _submitting ? null : _submit,
                  child: _submitting
                      ? const SizedBox(
                          width: 20,
                          height: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Text(_selected == null
                          ? loc.getString('common.submit')
                          : '${loc.getString('common.submit')} · ${formatYuan(_selected!.priceCents * _quantity)}'),
                ),
              ],
            ),
    );
  }
}

class _ItineraryCard extends StatelessWidget {
  const _ItineraryCard({required this.day});

  final ItineraryDay day;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: CircleAvatar(
          radius: 16,
          child: Text('D${day.day}', style: const TextStyle(fontSize: 12)),
        ),
        title: Text(day.title.isEmpty ? 'D${day.day}' : day.title),
        subtitle: day.description.isEmpty ? null : Text(day.description),
      ),
    );
  }
}

class _DateRow extends StatelessWidget {
  const _DateRow({required this.date, required this.selected, required this.onTap});

  final LineDate date;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final scheme = Theme.of(context).colorScheme;
    final available = date.available;
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      color: selected ? scheme.primaryContainer : null,
      child: ListTile(
        enabled: available,
        leading: Icon(
          selected ? Icons.radio_button_checked : Icons.radio_button_unchecked,
          color: selected ? scheme.primary : null,
        ),
        title: Text(date.date),
        subtitle: Text(
          available
              ? '${loc.getString('lines.seatsLeft')} ${date.seatsLeft}'
              : loc.getString('lines.soldOut'),
          style: TextStyle(color: available ? null : scheme.error),
        ),
        trailing: Text(
          formatYuan(date.priceCents),
          style: Theme.of(context).textTheme.titleSmall?.copyWith(color: scheme.primary),
        ),
        onTap: available ? onTap : null,
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
