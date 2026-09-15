package com.litebalance.app.ui

import androidx.compose.runtime.Composable
import com.litebalance.app.ui.theme.ThemeLiteBalance as AppleThemeLiteBalance

@Composable
fun ThemeLiteBalance(content: @Composable () -> Unit) {
    AppleThemeLiteBalance(content = content)
}
