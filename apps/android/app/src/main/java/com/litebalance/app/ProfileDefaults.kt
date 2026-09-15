package com.litebalance.app

import uniffi.litebalance.FfiUserInput

/** 默认演示档案：NASEM 官方例题（22F / 165 / 63 / low_active → 2275 kcal）。 */
fun demoUserInput(
    userId: String = "default_user",
    age: UByte = 22u,
    heightCm: Double = 165.0,
    weightKg: Double = 63.0,
    gender: String = "female",
    activity: String = "low_active",
): FfiUserInput = FfiUserInput(
    userId = userId,
    name = "轻衡用户",
    birthday = "2004-01-01",
    age = age,
    heightCm = heightCm,
    weightKg = weightKg,
    gender = gender,
    activityLevel = activity,
)
