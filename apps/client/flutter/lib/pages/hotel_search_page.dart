import 'package:dio/dio.dart';
import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/auth_service.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import '../widgets/login_dialog.dart';
import 'order_detail_page.dart';

/// 酒店搜索：城市（三字码）+ 星级筛选。
class HotelSearchPage extends StatefulWidget {
  const HotelSearchPage({super.key});

  @override
  State<HotelSearchPage> createState() => _HotelSearchPageState();
}

class _HotelSearchPageState extends State<HotelSearchPage> {
  final _city = TextEditingController();
  int? _star;
  List<Hotel> _hotels = [];
  bool _loading = false;
  bool _searched = false;
  String? _error;

  @override
  void dispose() {
    _city.dispose();
    super.dispose();
  }

  Future<void> _search() async {
    final loc = LocalizationService.instance;
    final city = _city.text.trim().toUpperCase();
    if (city.isEmpty) {
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('hotel.hint'))));
      return;
    }
    FocusScope.of(context).unfocus();
    setState(() {
      _loading = true;
      _error = null;
      _searched = true;
    });
    try {
      final hotels = await OrderService.instance.searchHotels(city: city, star: _star);
      if (!mounted) return;
      setState(() {
        _hotels = hotels;
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
      appBar: AppBar(title: Text(loc.getString('hotel.title'))),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _city,
                  textCapitalization: TextCapitalization.characters,
                  textInputAction: TextInputAction.search,
                  onSubmitted: (_) => _search(),
                  decoration: InputDecoration(
                    labelText: loc.getString('hotel.city'),
                    hintText: 'HKG',
                    prefixIcon: const Icon(Icons.location_city),
                    border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: DropdownButtonFormField<int?>(
                  initialValue: _star,
                  decoration: InputDecoration(
                    labelText: loc.getString('hotel.star'),
                    border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                  ),
                  items: [
                    DropdownMenuItem(value: null, child: Text(loc.getString('hotel.allStars'))),
                    for (final s in [5, 4, 3, 2, 1])
                      DropdownMenuItem(value: s, child: Text('$s${loc.getString('hotel.starUnit')}')),
                  ],
                  onChanged: (s) => setState(() => _star = s),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          FilledButton(onPressed: _loading ? null : _search, child: Text(loc.getString('hotel.search'))),
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
          else if (_searched && _hotels.isEmpty)
            Padding(
              padding: const EdgeInsets.all(24),
              child: Center(child: Text(loc.getString('hotel.noHotels'))),
            )
          else if (_searched)
            for (final h in _hotels) _HotelCard(hotel: h),
        ],
      ),
    );
  }
}

class _HotelCard extends StatelessWidget {
  const _HotelCard({required this.hotel});

  final Hotel hotel;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        leading: hotel.coverUrl.isNotEmpty
            ? ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: Image.network(
                  hotel.coverUrl,
                  width: 56,
                  height: 56,
                  fit: BoxFit.cover,
                  errorBuilder: (_, _, _) => const Icon(Icons.hotel, size: 32),
                ),
              )
            : const Icon(Icons.hotel, size: 40),
        title: Text(hotel.name),
        subtitle: Text('${'★' * hotel.star} · ${hotel.cityCode}'),
        onTap: () => Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => HotelDetailPage(
              hotel: Hotel(id: hotel.id, name: hotel.name, cityCode: hotel.cityCode, star: hotel.star, coverUrl: hotel.coverUrl),
            ),
          ),
        ),
      ),
    );
  }
}

/// 酒店详情：酒店信息 + 房型列表（房型名/价格/含早/库存），房型可预订。
class HotelDetailPage extends StatefulWidget {
  const HotelDetailPage({super.key, required this.hotel});

  final Hotel hotel;

  @override
  State<HotelDetailPage> createState() => _HotelDetailPageState();
}

class _HotelDetailPageState extends State<HotelDetailPage> {
  Hotel? _detail;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    try {
      final detail = await OrderService.instance.fetchHotel(widget.hotel.id);
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

  Future<void> _book(HotelRoom room) async {
    final loc = LocalizationService.instance;
    if (!AuthService.instance.isLoggedIn) {
      final ok = await showLoginDialog(context);
      if (!ok || !mounted) return;
    }
    DateTime? checkIn = DateTime.now();
    DateTime? checkOut = DateTime.now().add(const Duration(days: 1));
    var quantity = 1;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) {
          String fmt(DateTime d) =>
              '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
          Future<void> pick({required bool isIn}) async {
            final picked = await showDatePicker(
              context: context,
              initialDate: isIn ? checkIn! : checkOut!,
              firstDate: isIn ? DateTime.now() : checkIn!,
              lastDate: DateTime.now().add(const Duration(days: 365)),
            );
            if (picked != null) {
              setDialogState(() {
                if (isIn) {
                  checkIn = picked;
                  if (checkOut!.isBefore(picked)) checkOut = picked.add(const Duration(days: 1));
                } else {
                  checkOut = picked;
                }
              });
            }
          }

          return AlertDialog(
            title: Text(room.name),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () => pick(isIn: true),
                        icon: const Icon(Icons.calendar_month),
                        label: Text('${loc.getString('hotel.checkIn')} ${fmt(checkIn!)}'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () => pick(isIn: false),
                        icon: const Icon(Icons.calendar_month),
                        label: Text('${loc.getString('hotel.checkOut')} ${fmt(checkOut!)}'),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Text(loc.getString('common.quantity')),
                    const Spacer(),
                    IconButton(
                      onPressed: quantity > 1 ? () => setDialogState(() => quantity--) : null,
                      icon: const Icon(Icons.remove_circle_outline),
                    ),
                    Text('$quantity', style: Theme.of(context).textTheme.titleMedium),
                    IconButton(
                      onPressed: quantity < 9 ? () => setDialogState(() => quantity++) : null,
                      icon: const Icon(Icons.add_circle_outline),
                    ),
                  ],
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
                child: Text('${loc.getString('common.submit')} · ${formatYuan(room.priceCents * quantity)}'),
              ),
            ],
          );
        },
      ),
    );
    if (confirmed != true || !mounted) return;
    String fmt(DateTime d) =>
        '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
    try {
      final order = await OrderService.instance.createOrder(
        productId: room.id,
        lineDateId: 0,
        quantity: quantity,
        orderType: 3,
        checkIn: fmt(checkIn!),
        checkOut: fmt(checkOut!),
      );
      if (!mounted) return;
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('order.createSuccess'))));
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => OrderDetailPage(orderId: order.id)),
      );
    } on DioException catch (e) {
      if (!mounted) return;
      final msg = e.response?.statusCode == 409
          ? loc.getString('order.stockNotEnough')
          : loc.getString('common.error');
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
    } on Exception {
      if (!mounted) return;
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('common.error'))));
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final h = _detail ?? widget.hotel;
    return Scaffold(
      appBar: AppBar(title: Text(h.name)),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                if (h.coverUrl.isNotEmpty)
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: AspectRatio(
                      aspectRatio: 16 / 9,
                      child: Image.network(h.coverUrl, fit: BoxFit.cover,
                          errorBuilder: (_, _, _) => const SizedBox.shrink()),
                    ),
                  ),
                const SizedBox(height: 12),
                Text(h.name, style: Theme.of(context).textTheme.headlineSmall),
                const SizedBox(height: 8),
                Text('${'★' * h.star} · ${h.cityCode}'),
                const SizedBox(height: 24),
                Text(loc.getString('hotel.rooms'), style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                if (h.rooms.isEmpty)
                  Text(loc.getString('common.noData'))
                else
                  for (final room in h.rooms) _RoomCard(room: room, onBook: () => _book(room)),
              ],
            ),
    );
  }
}

class _RoomCard extends StatelessWidget {
  const _RoomCard({required this.room, required this.onBook});

  final HotelRoom room;
  final VoidCallback onBook;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final scheme = Theme.of(context).colorScheme;
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        title: Text(room.name),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                if (room.breakfast)
                  Text(loc.getString('hotel.breakfast'),
                      style: Theme.of(context).textTheme.labelSmall?.copyWith(color: scheme.primary)),
                const Spacer(),
                Text('${loc.getString('hotel.inventory')} ${room.inventory}',
                    style: Theme.of(context).textTheme.labelSmall),
              ],
            ),
          ],
        ),
        trailing: FilledButton.tonal(
          onPressed: room.available ? onBook : null,
          child: Text('${formatYuan(room.priceCents)} · ${loc.getString('hotel.book')}'),
        ),
      ),
    );
  }
}
