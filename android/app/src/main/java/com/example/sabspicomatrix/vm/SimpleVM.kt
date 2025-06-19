package com.example.sabspicomatrix.vm

import androidx.lifecycle.ViewModel
import com.example.sabspicomatrix.repository.BluetoothRepository
import com.example.sabspicomatrix.repository.BluetoothPowerState
import com.example.sabspicomatrix.repository.BluetoothState
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.StateFlow
import javax.inject.Inject

@HiltViewModel
class SimpleVM @Inject constructor(private val bluetoothRepository: BluetoothRepository) :
    ViewModel() {
    val hasBluetoothPermission = bluetoothRepository.hasBluetoothPermission
    val bluetoothPowerState: StateFlow<BluetoothState> =
        bluetoothRepository.bluetoothState

    fun update_permissions() {
        bluetoothRepository.update_permissions()
    }

    fun start_discovery() {
        bluetoothRepository.start_discovery()
    }
}