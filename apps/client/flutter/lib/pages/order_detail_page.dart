import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import 'payment_page.dart';

/// 订单详情：快照信息 + 状态 + 金额 + 下单时间，待支付可取消。
class OrderDetailPage extends StatefulWidget {
  const OrderDetailPage({super.key, required this.orderId});

  final int orderId;

  @override
  State<OrderDetailPage> createState() => _OrderDetailPageState();
}

class _OrderDetailPageState extends State<OrderDetailPage> {
  Order? _order;
  bool _loading = true;
  bool _failed = false;
  bool _cancelling = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _failed = false;
    });
    try {
      final order = await OrderService.instance.fetchOrder(widget.orderId);
      if (!mounted) return;
      setState(() {
        _order = order;
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

  Future<void> _cancel() async {
    final loc = LocalizationService.instance;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(loc.getString('order.cancel')),
        content: Text(loc.getString('order.cancelConfirm')),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: Text(loc.getString('common.cancel')),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: Text(loc.getString('order.cancel')),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    setState(() => _cancelling = true);
    try {
      final order = await OrderService.instance.cancelOrder(widget.orderId);
      if (!mounted) return;
      setState(() {
        _order = order;
        _cancelling = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(loc.getString('order.cancelSuccess'))),
      );
    } on Exception {
      if (!mounted) return;
      setState(() => _cancelling = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(loc.getString('common.error'))),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return Scaffold(
      appBar: AppBar(title: Text(loc.getString('order.title'))),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _failed || _order == null
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(loc.getString('common.loadFailed')),
                      TextButton(onPressed: _load, child: Text(loc.getString('common.retry'))),
                    ],
                  ),
                )
              : _buildBody(context, _order!, loc),
    );
  }

  Widget _buildBody(BuildContext context, Order order, loc) {
    final scheme = Theme.of(context).colorScheme;
    final lang = loc.locale.languageCode;
    final title = snapshotTitle(order.productSnapshot, lang);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      loc.getString(orderStatusKey(order.status)),
                      style: Theme.of(context)
                          .textTheme
                          .titleLarge
                          ?.copyWith(color: order.isPending ? scheme.error : scheme.primary),
                    ),
                    const Spacer(),
                    Text(formatYuan(order.amountCents), style: Theme.of(context).textTheme.headlineSmall),
                  ],
                ),
                const Divider(height: 24),
                _DetailRow(label: loc.getString('order.orderNo'), value: '${order.id}'),
                if (title.isNotEmpty) _DetailRow(label: loc.getString('order.product'), value: title),
                _DetailRow(label: loc.getString('order.amount'), value: formatYuan(order.amountCents)),
                if (order.createdAt.isNotEmpty)
                  _DetailRow(label: loc.getString('order.createdAt'), value: order.createdAt),
              ],
            ),
          ),
        ),
        if (order.isPending) ...[
          const SizedBox(height: 16),
          FilledButton.icon(
            onPressed: () async {
              await Navigator.of(context).push(
                MaterialPageRoute(builder: (_) => PaymentPage(orderId: order.id)),
              );
              _load();
            },
            icon: const Icon(Icons.payment),
            label: Text(loc.getString('payment.goPay')),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _cancelling ? null : _cancel,
            icon: _cancelling
                ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                : const Icon(Icons.close),
            label: Text(loc.getString('order.cancel')),
          ),
        ],
      ],
    );
  }
}

class _DetailRow extends StatelessWidget {
  const _DetailRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: Theme.of(context).textTheme.bodyMedium?.copyWith(color: Theme.of(context).colorScheme.outline)),
          const SizedBox(width: 12),
          Expanded(child: Text(value, textAlign: TextAlign.right)),
        ],
      ),
    );
  }
}
