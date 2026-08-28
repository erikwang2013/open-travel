import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/auth_service.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import '../widgets/login_dialog.dart';
import 'order_detail_page.dart';

/// 订单中心：未登录提示登录，已登录展示订单列表（状态徽标/金额/日期）。
class OrdersPage extends StatefulWidget {
  const OrdersPage({super.key});

  @override
  State<OrdersPage> createState() => _OrdersPageState();
}

class _OrdersPageState extends State<OrdersPage> {
  List<Order> _orders = [];
  bool _loading = false;
  bool _ready = false;
  bool _failed = false;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final auth = AuthService.instance;
    return ListenableBuilder(
      listenable: Listenable.merge([auth, loc]),
      builder: (context, _) {
        if (!auth.isLoggedIn) {
          _ready = false;
          return _LoginPrompt(onLogin: () async {
            await showLoginDialog(context);
          });
        }
        if (!_ready) {
          _ready = true;
          WidgetsBinding.instance.addPostFrameCallback((_) => _load());
        }
        return ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Text(loc.getString('order.title'), style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 12),
            if (_failed)
              Column(
                children: [
                  Text(loc.getString('common.loadFailed')),
                  TextButton(onPressed: _load, child: Text(loc.getString('common.retry'))),
                ],
              )
            else if (_loading && _orders.isEmpty)
              const Padding(
                padding: EdgeInsets.all(24),
                child: Center(child: CircularProgressIndicator()),
              )
            else if (_orders.isEmpty)
              Padding(
                padding: const EdgeInsets.all(24),
                child: Center(child: Text(loc.getString('common.noData'))),
              )
            else
              for (final order in _orders) _OrderCard(order: order, onChanged: _load),
          ],
        );
      },
    );
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _failed = false;
    });
    try {
      final page = await OrderService.instance.fetchOrders();
      if (!mounted) return;
      setState(() {
        _orders = page.items;
        _loading = false;
      });
    } on Exception {
      if (!mounted) return;
      setState(() {
        _failed = true;
        _loading = false;
      });
    }
  }
}

class _LoginPrompt extends StatelessWidget {
  const _LoginPrompt({required this.onLogin});

  final VoidCallback onLogin;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(loc.getString('profile.notLoggedIn')),
          const SizedBox(height: 12),
          FilledButton.icon(
            onPressed: onLogin,
            icon: const Icon(Icons.login),
            label: Text(loc.getString('profile.login')),
          ),
        ],
      ),
    );
  }
}

class _OrderCard extends StatelessWidget {
  const _OrderCard({required this.order, required this.onChanged});

  final Order order;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final scheme = Theme.of(context).colorScheme;
    final title = snapshotTitle(order.productSnapshot, loc.locale.languageCode);
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        leading: Icon(
          order.isPending ? Icons.pending_actions : Icons.card_travel,
          color: scheme.primary,
        ),
        title: Text(title.isEmpty ? '#${order.id}' : title),
        subtitle: Text(
          order.createdAt.isEmpty
              ? '#${order.id} · ${formatYuan(order.amountCents)}'
              : '${order.createdAt} · ${formatYuan(order.amountCents)}',
        ),
        trailing: Text(
          loc.getString(orderStatusKey(order.status)),
          style: TextStyle(
            color: order.isPending ? scheme.error : scheme.primary,
            fontWeight: FontWeight.w600,
          ),
        ),
        onTap: () async {
          await Navigator.of(context).push(
            MaterialPageRoute(builder: (_) => OrderDetailPage(orderId: order.id)),
          );
          onChanged();
        },
      ),
    );
  }
}
