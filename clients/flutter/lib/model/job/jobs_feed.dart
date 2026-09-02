import 'package:shadowmask/model/job/job.dart';
import 'package:shadowmask/view/page.dart';

class JobsFeed {
  const JobsFeed({
    required this.page,
    required this.activeTotal,
    required this.allTotal,
  });

  final Paged<Job> page;
  final int activeTotal;
  final int allTotal;

  factory JobsFeed.fromJson(Map<String, dynamic> json) => JobsFeed(
    page: Paged<Job>.fromJson(json, Job.fromJson),
    activeTotal: (json['active_total'] as num?)?.toInt() ?? 0,
    allTotal: (json['all_total'] as num?)?.toInt() ?? 0,
  );
}
