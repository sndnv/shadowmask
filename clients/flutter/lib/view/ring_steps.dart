const int kRingSteps = 40;

int ringFilledSteps(int percent) =>
    (percent.clamp(0, 100) * kRingSteps / 100).ceil();
