package com.example.sabspicomatrix.repository

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import androidx.core.content.ContextCompat.getSystemService
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combineTransform
import javax.inject.Inject

sealed class BluetoothState {
    object Unknown : BluetoothState()
    object Off : BluetoothState()
    object TurningOn : BluetoothState()
    object On : BluetoothState()
    object TurningOff : BluetoothState()
}

class BluetoothRepository @Inject constructor(
    private val permissionRepository: PermissionRepository,
    val appContext: Context
) {
    private val stateChangedReceiver = object : BroadcastReceiver() {
        override fun onReceive(
            context: Context?,
            intent: Intent?
        ) {
            val action = intent?.action.toString()
            when (action) {
                BluetoothAdapter.ACTION_STATE_CHANGED -> {
                    intent?.getIntExtra(BluetoothAdapter.EXTRA_STATE, -1)
                        ?.apply { update_state_from_intent(this) }

                }
            }
        }
    }
    private var intentRegistered: Boolean = false
    private var manager: BluetoothManager =
        getSystemService(appContext, BluetoothManager::class.java)!!
    private val currentThread: BluetoothThread? = null
    private val _bluetoothState: MutableStateFlow<BluetoothState> =
        MutableStateFlow(BluetoothState.Unknown)

    private fun update_state_from_intent(newState: Int) {
        when (newState) {
            BluetoothAdapter.STATE_OFF -> _bluetoothState.tryEmit(BluetoothState.Off)
            BluetoothAdapter.STATE_TURNING_ON -> _bluetoothState.tryEmit(BluetoothState.TurningOn)
            BluetoothAdapter.STATE_ON -> _bluetoothState.tryEmit(BluetoothState.On)
            BluetoothAdapter.STATE_TURNING_OFF -> _bluetoothState.tryEmit(BluetoothState.TurningOff)
        }
    }

    private fun register_bluetooth_intent() {
        if (!intentRegistered) {
            appContext.registerReceiver(
                stateChangedReceiver,
                IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED)
            )
            intentRegistered = true
        }
    }


    init {
        register_bluetooth_intent()
    }

    val hasBluetoothPermission: Flow<Boolean> =
        permissionRepository.btConnectPermission.combineTransform(
            permissionRepository.btScanPermission,
            { p1, p2 ->
                emit(p1 && p2)
            })

    val bluetoothState: StateFlow<BluetoothState> = _bluetoothState

    fun update_permissions() = permissionRepository.update_permissions()
}