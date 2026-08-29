import 'package:flutter/material.dart';

import '../api.dart';

const orderTypeLabels = {1: '线路', 2: '航班', 3: '酒店'};
const orderStatusLabels = {0: '待支付', 1: '已支付', 2: '已确认', 3: '已完成', 4: '已取消'};

class Order {
  Order.fromJson(Map<String, dynamic> j)
      : id = j['id'] as int,
        email = (j['email'] ?? '') as String,
        orderType = (j['order_type'] ?? 0) as int,
        productId = (j['product_id'] ?? 0) as int,
        status = (j['status'] ?? 0) as int,
        amountCents = (j['amount_cents'] ?? 0) as int,
        snapshot = (j['snapshot'] is Map) ? (j['snapshot'] as Map).cast<String, dynamic>() : <String, dynamic>{},
        createdAt = (j['created_at'] ?? '') as String;

  final int id;
  final String email;
  final int orderType;
  final int productId;
  final int status;
  final int amountCents;
  final Map<String, dynamic> snapshot;
  final String createdAt;
}

class OrdersPage extends StatefulWidget {
  const OrdersPage({super.key});

  @override
  State<OrdersPage> createState() => _OrdersPageState();
}

class _OrdersPageState extends State<OrdersPage> {
  List<Order> _list = [];
  int _total = 0;
  int _page = 1;
  static const _pageSize = 10;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await Api.get('/api/admin/orders',
          {'page': '$_page', 'page_size': '$_pageSize'});
      setState(() {
        _list = (data['items'] as List)
            .map((e) => Order.fromJson(e as Map<String, dynamic>))
            .toList();
        _total = data['total'] as int;
        _loading = false;
      });
    } on ApiException catch (e) {
      setState(() {
        _error = e.message;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _error = '网络错误：$e';
        _loading = false;
      });
    }
  }

  void _showSnack(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
  }

  Future<void> _confirmRefund(Order o) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('退款确认'),
        content: Text('确定对订单 #${o.id}（¥${(o.amountCents / 100).toStringAsFixed(2)}）退款吗？将回补对应商品库存。'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('退款')),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await Api.post('/api/admin/orders/${o.id}/refund', {});
      _showSnack('已退款');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    } catch (e) {
      _showSnack('网络错误：$e');
    }
  }

  void _showDetail(Order o) {
    final s = o.snapshot;
    final date = (s['depart_date'] ?? s['depart_at'] ?? '') as String;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('订单详情 #${o.id}'),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              _kv('用户', o.email),
              _kv('类型', '${orderTypeLabels[o.orderType] ?? o.orderType}'),
              _kv('商品 ID', '${o.productId}'),
              _kv('金额', '¥${(o.amountCents / 100).toStringAsFixed(2)}'),
              _kv('状态', orderStatusLabels[o.status] ?? '${o.status}'),
              _kv('下单时间', o.createdAt),
              const Divider(),
              _kv('商品', '${s['title'] ?? '-'}'),
              if (date.isNotEmpty) _kv('日期', date),
              if (s['quantity'] != null) _kv('数量', '${s['quantity']}'),
              if (s['check_in'] != null) _kv('入住', '${s['check_in']}'),
              if (s['check_out'] != null) _kv('离店', '${s['check_out']}'),
            ],
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('关闭')),
        ],
      ),
    );
  }

  Widget _kv(String k, String v) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(width: 80, child: Text(k, style: const TextStyle(color: Colors.grey))),
            Expanded(child: Text(v)),
          ],
        ),
      );

  @override
  Widget build(BuildContext context) {
    final pages = (_total + _pageSize - 1) ~/ _pageSize;
    return Column(
      children: [
        Expanded(
          child: _loading
              ? const Center(child: CircularProgressIndicator())
              : _error != null
                  ? Center(child: Text(_error!))
                  : SingleChildScrollView(
                      child: DataTable(
                        columns: const [
                          DataColumn(label: Text('订单号')),
                          DataColumn(label: Text('用户')),
                          DataColumn(label: Text('类型')),
                          DataColumn(label: Text('金额')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('时间')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((o) => DataRow(cells: [
                                  DataCell(Text('#${o.id}')),
                                  DataCell(Text(o.email)),
                                  DataCell(Text(orderTypeLabels[o.orderType] ?? '${o.orderType}')),
                                  DataCell(Text(
                                      '¥${(o.amountCents / 100).toStringAsFixed(2)}')),
                                  DataCell(Text(
                                      orderStatusLabels[o.status] ?? '${o.status}')),
                                  DataCell(Text(o.createdAt)),
                                  DataCell(Row(children: [
                                    IconButton(
                                        tooltip: '详情',
                                        icon: const Icon(Icons.visibility_outlined),
                                        onPressed: () => _showDetail(o)),
                                    IconButton(
                                        tooltip: '退款',
                                        icon: const Icon(Icons.currency_exchange),
                                        onPressed: (o.status == 1 || o.status == 2)
                                            ? () => _confirmRefund(o)
                                            : null),
                                  ])),
                                ]))
                            .toList(),
                      ),
                    ),
        ),
        Padding(
          padding: const EdgeInsets.all(8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              IconButton(
                icon: const Icon(Icons.chevron_left),
                onPressed: _page <= 1
                    ? null
                    : () {
                        setState(() => _page--);
                        _load();
                      },
              ),
              Text('第 $_page / ${pages < 1 ? 1 : pages} 页（共 $_total 条）'),
              IconButton(
                icon: const Icon(Icons.chevron_right),
                onPressed: _page >= pages
                    ? null
                    : () {
                        setState(() => _page++);
                        _load();
                      },
              ),
            ],
          ),
        ),
      ],
    );
  }
}
