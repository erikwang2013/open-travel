import 'package:flutter/material.dart';

import '../api.dart';

const _statusLabels = {0: '待支付', 1: '已支付', 2: '已确认', 3: '已完成', 4: '已取消'};

class DashboardPage extends StatefulWidget {
  const DashboardPage({super.key});

  @override
  State<DashboardPage> createState() => _DashboardPageState();
}

class _DashboardPageState extends State<DashboardPage> {
  bool _loading = true;
  String? _error;
  Map<String, dynamic> _overview = {};
  List<Map<String, dynamic>> _trend = [];
  List<Map<String, dynamic>> _destinations = [];
  List<Map<String, dynamic>> _lines = [];

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
      final results = await Future.wait([
        Api.get('/api/admin/stats/overview'),
        Api.get('/api/admin/stats/trend'),
        Api.get('/api/admin/stats/top'),
      ]);
      setState(() {
        _overview = (results[0] as Map).cast<String, dynamic>();
        _trend = ((results[1]['items'] ?? []) as List)
            .map((e) => (e as Map).cast<String, dynamic>())
            .toList();
        _destinations = ((results[2]['top_destinations'] ?? []) as List)
            .map((e) => (e as Map).cast<String, dynamic>())
            .toList();
        _lines = ((results[2]['top_lines'] ?? []) as List)
            .map((e) => (e as Map).cast<String, dynamic>())
            .toList();
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

  String _fmtCents(int cents) {
    final yuan = cents / 100.0;
    return '¥${yuan.toStringAsFixed(2)}';
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text('加载失败：$_error'),
            const SizedBox(height: 12),
            FilledButton(onPressed: _load, child: const Text('重试')),
          ],
        ),
      );
    }
    final statusCounts = (_overview['status_counts'] as Map? ?? {})
        .map((k, v) => MapEntry(int.parse(k), v as int));
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(
            children: [
              _MetricCard(
                  label: '总订单数',
                  value: '${_overview['total_orders'] ?? 0}',
                  color: Colors.blue),
              _MetricCard(
                  label: 'GMV（已支付）',
                  value: _fmtCents(_overview['gmv_cents'] as int? ?? 0),
                  color: Colors.green),
              _MetricCard(
                  label: '支付转化率',
                  value: '${_overview['conversion_rate'] ?? 0.0}%',
                  color: Colors.orange),
            ],
          ),
          const SizedBox(height: 16),
          _Card(
            title: '近 7 天订单趋势',
            child: SizedBox(height: 200, child: _TrendChart(data: _trend)),
          ),
          const SizedBox(height: 16),
          Row(
            children: [
              Expanded(
                child: _Card(
                  title: '订单状态分布',
                  child: _StatusBar(
                      counts: statusCounts, total: _overview['total_orders'] as int? ?? 0),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: _Card(
                  title: 'Top 5 目的地',
                  child: _TopList(items: _destinations),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Card(title: 'Top 5 线路', child: _TopList(items: _lines)),
        ],
      ),
    );
  }
}

class _MetricCard extends StatelessWidget {
  const _MetricCard({required this.label, required this.value, required this.color});

  final String label;
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Card(
        color: color.withValues(alpha: 0.08),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label, style: const TextStyle(fontSize: 13, color: Colors.grey)),
              const SizedBox(height: 8),
              Text(value,
                  style: TextStyle(
                      fontSize: 22, fontWeight: FontWeight.bold, color: color)),
            ],
          ),
        ),
      ),
    );
  }
}

class _Card extends StatelessWidget {
  const _Card({required this.title, required this.child});

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600)),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}

class _TrendChart extends StatelessWidget {
  const _TrendChart({required this.data});

  final List<Map<String, dynamic>> data;

  @override
  Widget build(BuildContext context) {
    if (data.isEmpty) return const Center(child: Text('暂无数据'));
    return CustomPaint(
      size: Size.infinite,
      painter: _TrendPainter(data: data),
    );
  }
}

class _TrendPainter extends CustomPainter {
  _TrendPainter({required this.data});

  final List<Map<String, dynamic>> data;
  static const _color = Color(0xFF1E88E5);

  @override
  void paint(Canvas canvas, Size size) {
    if (data.isEmpty) return;
    final maxV = data.map((e) => e['orders'] as int).fold(1, (a, b) => a > b ? a : b);
    const padL = 36.0, padB = 26.0, padT = 12.0;
    final chartW = size.width - padL - 8;
    final chartH = size.height - padT - padB;
    final grid = _GridPainter();
    grid.paint(canvas, size);

    // 柱状图 + 折线
    final n = data.length;
    final step = chartW / n;
    final path = Path();
    for (var i = 0; i < n; i++) {
      final v = data[i]['orders'] as int;
      final x = padL + step * i + step / 2;
      final h = v == 0 ? 2.0 : chartH * v / maxV;
      final rect = Rect.fromLTWH(x - step * 0.25, size.height - padB - h, step * 0.5, h);
      canvas.drawRRect(
          RRect.fromRectAndRadius(rect, const Radius.circular(3)),
          Paint()..color = _color.withValues(alpha: 0.35));
      if (i == 0) {
        path.moveTo(x, size.height - padB - h);
      } else {
        path.lineTo(x, size.height - padB - h);
      }
      // 日期标签（间隔显示避免拥挤）
      if (i % 2 == 0 || i == n - 1) {
        final tp = TextPainter(
          text: TextSpan(
              text: (data[i]['day'] as String).substring(5),
              style: const TextStyle(fontSize: 10, color: Colors.grey)),
          textDirection: TextDirection.ltr,
        )..layout();
        tp.paint(canvas, Offset(x - tp.width / 2, size.height - padB + 4));
      }
    }
    canvas.drawPath(
      path,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2
        ..color = _color,
    );
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

class _GridPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final gridPaint = Paint()
      ..color = Colors.grey.withValues(alpha: 0.15)
      ..strokeWidth = 1;
    for (var i = 0; i <= 4; i++) {
      final y = 12 + (size.height - 38) * i / 4;
      canvas.drawLine(Offset(36, y), Offset(size.width - 8, y), gridPaint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _StatusBar extends StatelessWidget {
  const _StatusBar({required this.counts, required this.total});

  final Map<int, int> counts;
  final int total;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: _statusLabels.entries.map((e) {
        final v = counts[e.key] ?? 0;
        final frac = total == 0 ? 0.0 : v / total;
        return Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Row(
            children: [
              SizedBox(width: 52, child: Text(e.value, style: const TextStyle(fontSize: 12))),
              Expanded(
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(3),
                  child: LinearProgressIndicator(value: frac, minHeight: 10),
                ),
              ),
              const SizedBox(width: 8),
              Text('$v', style: const TextStyle(fontSize: 12)),
            ],
          ),
        );
      }).toList(),
    );
  }
}

class _TopList extends StatelessWidget {
  const _TopList({required this.items});

  final List<Map<String, dynamic>> items;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const Padding(
          padding: EdgeInsets.all(8), child: Center(child: Text('暂无数据')));
    }
    final maxV = items.map((e) => e['orders'] as int).fold(1, (a, b) => a > b ? a : b);
    return Column(
      children: items.map((e) {
        final v = e['orders'] as int;
        return Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Row(
            children: [
              SizedBox(width: 90, child: Text(e['name'] as String,
                  overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 12))),
              Expanded(
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(3),
                  child: LinearProgressIndicator(
                      value: v / maxV, minHeight: 10, color: Colors.teal),
                ),
              ),
              const SizedBox(width: 8),
              Text('$v', style: const TextStyle(fontSize: 12)),
            ],
          ),
        );
      }).toList(),
    );
  }
}
