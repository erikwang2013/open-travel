import 'dart:async';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../models/travel_models.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import 'order_detail_page.dart';

/// 支付引导：渠道列表 → 发起支付 → 打开沙箱收银台 → 轮询订单状态（最多 60s）。
class PaymentPage extends StatefulWidget {
  const PaymentPage({super.key, required this.orderId});

  final int orderId;

  @override
  State<PaymentPage> createState() => _PaymentPageState();
}

class _PaymentPageState extends State<PaymentPage> {
  List<PaymentChannel> _channels = [];
  bool _loading = true;
  bool _failed = false;
  bool _paying = false;
  bool _polling = false;
  Timer? _timer;
  int _elapsed = 0;

  static const _pollInterval = Duration(seconds: 2);
  static const _pollLimit = 30; // 30 * 2s = 60s

  @override
  void initState() {
    super.initState();
    _loadChannels();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _loadChannels() async {
    setState(() {
      _loading = true;
      _failed = false;
    });
    try {
      final channels = await OrderService.instance.fetchPaymentChannels();
      if (!mounted) return;
      setState(() {
        _channels = [for (final c in channels) if (c.enabled) c];
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

  Future<void> _pay(PaymentChannel channel) async {
    final loc = LocalizationService.instance;
    setState(() => _paying = true);
    try {
      final result = await OrderService.instance
          .createPayment(orderId: widget.orderId, channelCode: channel.channelCode);
      if (!mounted) return;
      if (result.checkoutUrl.isNotEmpty) {
        await launchUrl(Uri.parse(result.checkoutUrl),
            mode: LaunchMode.externalApplication);
      }
      setState(() => _paying = false);
      _startPolling();
    } on Exception {
      if (!mounted) return;
      setState(() => _paying = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('common.error'))));
    }
  }

  void _startPolling() {
    setState(() {
      _polling = true;
      _elapsed = 0;
    });
    _timer = Timer.periodic(_pollInterval, (_) => _poll());
  }

  Future<void> _poll() async {
    final loc = LocalizationService.instance;
    if (_elapsed >= _pollLimit) {
      _timer?.cancel();
      if (!mounted) return;
      setState(() => _polling = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('payment.timeout'))));
      return;
    }
    _elapsed++;
    try {
      final order = await OrderService.instance.fetchOrder(widget.orderId);
      if (!mounted || order.status != 1) return;
      _timer?.cancel();
      setState(() => _polling = false);
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(loc.getString('payment.success'))));
      Navigator.of(context).pushReplacement(
        MaterialPageRoute(builder: (_) => OrderDetailPage(orderId: order.id)),
      );
    } on Exception {
      // 轮询失败静默重试，超时由上限兜底
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return Scaffold(
      appBar: AppBar(title: Text(loc.getString('payment.title'))),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _failed
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(loc.getString('common.loadFailed')),
                      TextButton(onPressed: _loadChannels, child: Text(loc.getString('common.retry'))),
                    ],
                  ),
                )
              : _channels.isEmpty
                  ? Center(child: Text(loc.getString('common.noData')))
                  : ListView(
                      padding: const EdgeInsets.all(16),
                      children: [
                        Text(loc.getString('payment.channels'),
                            style: Theme.of(context).textTheme.titleLarge),
                        const SizedBox(height: 12),
                        for (final c in _channels)
                          Card(
                            margin: const EdgeInsets.only(bottom: 12),
                            child: ListTile(
                              leading: const Icon(Icons.payment),
                              title: Text(c.name.isEmpty ? c.channelCode : c.name),
                              subtitle: c.type.isEmpty ? null : Text(c.type),
                              trailing: _polling
                                  ? const SizedBox(
                                      width: 20,
                                      height: 20,
                                      child: CircularProgressIndicator(strokeWidth: 2),
                                    )
                                  : _paying
                                      ? const Icon(Icons.hourglass_top)
                                      : const Icon(Icons.chevron_right),
                              onTap: _paying || _polling ? null : () => _pay(c),
                            ),
                          ),
                        if (_polling) ...[
                          const SizedBox(height: 12),
                          Text(loc.getString('payment.checkoutHint'),
                              style: TextStyle(color: Theme.of(context).colorScheme.outline)),
                        ],
                      ],
                    ),
    );
  }
}
