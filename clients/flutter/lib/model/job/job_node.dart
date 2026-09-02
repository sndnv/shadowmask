import 'package:shadowmask/model/job/job.dart';

class JobNode {
  const JobNode({required this.job, required this.depth});

  final Job job;
  final int depth;

  factory JobNode.fromJson(Map<String, dynamic> json) => JobNode(
    job: Job.fromJson(json),
    depth: (json['depth'] as num?)?.toInt() ?? 0,
  );
}
