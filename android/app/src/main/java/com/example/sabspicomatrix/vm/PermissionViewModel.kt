package com.example.sabspicomatrix.vm

import androidx.lifecycle.ViewModel
import com.example.sabspicomatrix.repository.BluetoothRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject

/**
 * A ViewModel for checking permissions
 */
@HiltViewModel
class PermissionViewModel @Inject constructor(private val bluetoothRepository: BluetoothRepository) :
    ViewModel() {
    val hasBluetoothPermission = bluetoothRepository.hasBluetoothPermission
    fun update_permissions() {
        bluetoothRepository.update_permissions()
    }
}