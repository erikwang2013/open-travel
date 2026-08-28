import 'package:flutter/material.dart';

import '../services/localization_service.dart';

class BookingListPage extends StatelessWidget {
  const BookingListPage({super.key});

  static const List<Map<String, String>> _placeholderBookings = [
    {'title': 'Tokyo — 3 nights', 'date': '2026-09-15', 'statusKey': 'booking.status.paid'},
    {'title': 'Paris — 2 nights', 'date': '2026-10-01', 'statusKey': 'booking.status.pending'},
  ];

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          loc.getString('booking.title'),
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 12),
        if (_placeholderBookings.isEmpty)
          Text(loc.getString('common.noData'))
        else
          for (final b in _placeholderBookings)
            Card(
              child: ListTile(
                leading: const Icon(Icons.card_travel),
                title: Text(b['title']!),
                subtitle: Text(b['date']!),
                trailing: Text(loc.getString(b['statusKey']!)),
              ),
            ),
      ],
    );
  }
}
