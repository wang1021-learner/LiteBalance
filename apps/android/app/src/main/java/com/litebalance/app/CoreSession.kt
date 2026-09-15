package com.litebalance.app

import android.content.Context
import uniffi.litebalance.LiteBalanceSession
import java.io.File

/**
 * 进程内单例会话：打开沙箱 SQLite，供各页面在 IO 线程调用。
 */
object CoreSession {
    @Volatile
    private var session: LiteBalanceSession? = null

    fun get(context: Context): LiteBalanceSession {
        session?.let { return it }
        synchronized(this) {
            session?.let { return it }
            val dir = context.applicationContext.filesDir
            val db = File(dir, "litebalance.db").absolutePath
            val cache = File(dir, "food_cache.db").absolutePath
            val opened = LiteBalanceSession.open(db, cache)
            session = opened
            return opened
        }
    }
}
