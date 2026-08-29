import 'package:dio/dio.dart';
import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/auth_service.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import '../widgets/login_dialog.dart';
import 'order_detail_page.dart';

/// 机票搜索：起降城市（IATA 三字码）+ 日期 + 舱位过滤。
class FlightSearchPage extends StatefulWidget {
  const FlightSearchPage({super.key});

  @override
  State<FlightSearchPage> createState() => _FlightSearchPageState();
}

class _FlightSearchPageState extends State<FlightSearchPage> {
  final _from = TextEditingController();
  final _to = TextEditingController();
  DateTime? _date;
  int? _cabin;
  List<Flight> _flights = [];
  bool _loading = false;
  bool _searched = false;
  String? _error;

  @override
  void dispose() {
    _from.dispose();
    _to.dispose();
    super.dispose();
  }

  Future<void> _pickDate() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _date ?? DateTime.now(),
      firstDate: DateTime.now(),
      lastDate: DateTime.now().add(const Duration(days: 365)),
    );
    if (picked != null) setState(() => _date = picked);
  }

  String? get _dateText => _date == null
      ? null
      : '${_date!.year}-${_date!.month.toString().padLeft(2, '0')}-${_date!.day.toString().padLeft(2, '0')}';

  Future<void> _search() async {
    final loc = LocalizationService.instance;
    final from = _from.text.trim().toUpperCase();
    final to = _to.text.trim().toUpperCase();
    if (from.isEmpty || to.isEmpty) {
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('flight.hint'))));
      return;
    }
    FocusScope.of(context).unfocus();
    setState(() {
      _loading = true;
      _error = null;
      _searched = true;
    });
    try {
      final flights = await OrderService.instance.searchFlights(
        from: from,
        to: to,
        departDate: _dateText,
        cabin: _cabin,
      );
      if (!mounted) return;
      setState(() {
        _flights = flights;
        _loading = false;
      });
    } on Exception {
      if (!mounted) return;
      setState(() {
        _error = loc.getString('common.loadFailed');
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return Scaffold(
      appBar: AppBar(title: Text(loc.getString('flight.title'))),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _from,
                  textCapitalization: TextCapitalization.characters,
                  decoration: InputDecoration(
                    labelText: loc.getString('flight.from'),
                    hintText: 'HND',
                    border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              const Icon(Icons.arrow_forward),
              const SizedBox(width: 8),
              Expanded(
                child: TextField(
                  controller: _to,
                  textCapitalization: TextCapitalization.characters,
                  decoration: InputDecoration(
                    labelText: loc.getString('flight.to'),
                    hintText: 'HKG',
                    border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: OutlinedButton.icon(
                  onPressed: _pickDate,
                  icon: const Icon(Icons.calendar_month),
                  label: Text(_dateText ?? loc.getString('flight.date')),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: DropdownButtonFormField<int?>(
                  initialValue: _cabin,
                  decoration: InputDecoration(
                    labelText: loc.getString('flight.cabin'),
                    border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                  ),
                  items: [
                    DropdownMenuItem(value: null, child: Text(loc.getString('flight.cabin.all'))),
                    for (final c in [0, 1, 2])
                      DropdownMenuItem(value: c, child: Text(loc.getString(flightCabinKey(c)))),
                  ],
                  onChanged: (c) => setState(() => _cabin = c),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          FilledButton(onPressed: _loading ? null : _search, child: Text(loc.getString('flight.search'))),
          const SizedBox(height: 16),
          if (_loading)
            const Padding(
              padding: EdgeInsets.all(24),
              child: Center(child: CircularProgressIndicator()),
            )
          else if (_error != null)
            Column(
              children: [
                Text(_error!),
                TextButton(onPressed: _search, child: Text(loc.getString('common.retry'))),
              ],
            )
          else if (_searched && _flights.isEmpty)
            Padding(
              padding: const EdgeInsets.all(24),
              child: Center(child: Text(loc.getString('flight.noFlights'))),
            )
          else if (_searched)
            for (final f in _flights) _FlightCard(flight: f),
        ],
      ),
    );
  }
}

class _FlightCard extends StatelessWidget {
  const _FlightCard({required this.flight});

  final Flight flight;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final scheme = Theme.of(context).colorScheme;
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        title: Text('${flight.airline} ${flight.flightNo}'),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${flight.fromCode} → ${flight.toCode}'),
            Row(
              children: [
                Text(loc.getString(flightCabinKey(flight.cabin)),
                    style: Theme.of(context).textTheme.labelSmall),
                const Spacer(),
                Text('${loc.getString('flight.seatsLeft')} ${flight.seatsLeft}',
                    style: Theme.of(context).textTheme.labelSmall),
              ],
            ),
          ],
        ),
        trailing: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            Text(
              formatYuan(flight.priceCents),
              style: Theme.of(context).textTheme.titleSmall?.copyWith(color: scheme.primary),
            ),
            Text('${flight.departAt} - ${flight.arriveAt}',
                style: Theme.of(context).textTheme.labelSmall),
          ],
        ),
        onTap: () => Navigator.of(context).push(
          MaterialPageRoute(builder: (_) => FlightDetailPage(flight: flight)),
        ),
      ),
    );
  }
}

/// 航班详情：完整信息 + 数量选择 + 提交订单（order_type=2）。
class FlightDetailPage extends StatefulWidget {
  const FlightDetailPage({super.key, required this.flight});

  final Flight flight;

  @override
  State<FlightDetailPage> createState() => _FlightDetailPageState();
}

class _FlightDetailPageState extends State<FlightDetailPage> {
  Flight? _detail;
  bool _loading = true;
  bool _submitting = false;
  int _quantity = 1;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final detail = await OrderService.instance.fetchFlight(widget.flight.id);
      if (!mounted) return;
      setState(() {
        _detail = detail;
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
        productId: widget.flight.id,
        lineDateId: 0,
        quantity: _quantity,
        orderType: 2,
      );
      if (!mounted) return;
      setState(() => _submitting = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('order.createSuccess'))));
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
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('common.error'))));
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final f = _detail ?? widget.flight;
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text('${f.airline} ${f.flightNo}')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('${f.fromCode} → ${f.toCode}',
                            style: Theme.of(context).textTheme.headlineSmall),
                        const SizedBox(height: 8),
                        Row(
                          children: [
                            _InfoChip(icon: Icons.schedule, text: '${f.departAt} - ${f.arriveAt}'),
                            const SizedBox(width: 16),
                            _InfoChip(icon: Icons.chair, text: loc.getString(flightCabinKey(f.cabin))),
                          ],
                        ),
                        const SizedBox(height: 8),
                        Row(
                          children: [
                            _InfoChip(icon: Icons.flight_takeoff, text: '${f.airline} ${f.flightNo}'),
                            const Spacer(),
                            Text(
                              formatYuan(f.priceCents),
                              style: Theme.of(context).textTheme.headlineSmall?.copyWith(color: scheme.primary),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
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
                  onPressed: f.soldOut || _submitting ? null : _submit,
                  child: _submitting
                      ? const SizedBox(
                          width: 20,
                          height: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Text('${loc.getString('common.submit')} · ${formatYuan(f.priceCents * _quantity)}'),
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
