package com.example.sabspicomatrix.repository

import android.util.Log
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.newCoroutineContext
import kotlin.coroutines.CoroutineContext

class BluetoothJob(scope: CoroutineScope) {
    private val job = CoroutineScope(Dispatchers.Default).launch {
        try {
            while (true) {
                Log.d("BluetoothJob", "BTJ running")
                delay(1000)
            }
        } catch (e: CancellationException) {
            Log.d("BluetoothJob", "Got cancelled")
        }
    }

    fun cancel() {
        job.cancel(CancellationException("Yo, get cancelled motherfucka"))
        job.isCancelled
    }
}