package me.weishu.kernelsu.ui.screen.modulerepo

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.unit.dp
import me.weishu.kernelsu.ui.viewmodel.ModuleSourcesState
import me.weishu.kernelsu.ui.viewmodel.ModuleSourcesViewModel

@Composable
fun ModuleSourcePicker(state: ModuleSourcesState, model: ModuleSourcesViewModel) {
    var expanded by remember { mutableStateOf(false) }
    var manage by remember { mutableStateOf(false) }
    var input by remember { mutableStateOf("") }
    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.weight(1f)) {
            TextButton(enabled = !state.busy, onClick = { expanded = true }) {
                Text(if (state.active == "official") "KernelSU catalogue ▾" else "${state.active} ▾")
            }
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                state.sources.forEach { source ->
                    DropdownMenuItem(text = { Text(source) }, onClick = { expanded = false; model.select(source) })
                }
                DropdownMenuItem(text = { Text("KernelSU catalogue") }, onClick = { expanded = false; model.select("official") })
            }
        }
        TextButton(enabled = !state.busy, onClick = { manage = true }) { Text("Sources") }
    }
    if (manage) AlertDialog(
        onDismissRequest = { manage = false },
        title = { Text("Module sources") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Paste a public GitHub repository to browse its modules.")
                LazyColumn(Modifier.heightIn(max = 200.dp)) {
                    items(state.sources, key = { it }) { source ->
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            Text(source, Modifier.weight(1f))
                            TextButton(enabled = !state.busy, onClick = { model.remove(source) }) { Text("Remove") }
                        }
                    }
                }
                OutlinedTextField(value = input, onValueChange = { input = it }, singleLine = true,
                    label = { Text("GitHub repository") }, placeholder = { Text("https://github.com/owner/repo") })
                state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(enabled = !state.busy && input.isNotBlank(), onClick = {
            if (model.add(input)) { input = ""; manage = false }
        }) { Text("Add source") } },
        dismissButton = { TextButton(onClick = { manage = false }) { Text("Done") } },
    )
}

@Composable
fun ModuleSourcesPanel(state: ModuleSourcesState, model: ModuleSourcesViewModel,
    installed: Map<String, Int>, onBack: () -> Unit) {
    var confirm by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().padding(horizontal = 16.dp).navigationBarsPadding()) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = onBack) { Text("Back") }
            Text("Modules", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            TextButton(enabled = !state.busy, onClick = model::refresh) { Text("Refresh") }
        }
        if (state.busy) {
            LinearProgressIndicator(Modifier.fillMaxWidth())
            Text(state.progress ?: "Preparing packages", Modifier.padding(vertical = 8.dp))
        }
        state.error?.let { Text(it, Modifier.padding(vertical = 8.dp), color = MaterialTheme.colorScheme.error) }
        if (!state.busy && state.packages.isEmpty() && state.error == null) {
            Text("No stable module ZIPs found. This source needs release assets or a modules.json catalogue.")
        }
        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.packages, key = { it.key }) { item ->
                val current = item.id?.let(installed::get)
                val upToDate = current != null && item.versionCode != null && current >= item.versionCode
                val enabled = !state.busy && !upToDate
                Card(Modifier.fillMaxWidth().clickable(enabled = enabled) { model.toggle(item.key) }) {
                    Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = item.key in state.selected, enabled = enabled,
                            onCheckedChange = { model.toggle(item.key) })
                        Column(Modifier.weight(1f)) {
                            Text(item.name, style = MaterialTheme.typography.titleMedium)
                            Text(item.version + if (upToDate) " · Installed" else if (current != null) " · Update available" else "")
                            if (item.description.isNotBlank()) Text(item.description, style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
        }
        Button(enabled = !state.busy && state.selected.isNotEmpty(), onClick = { confirm = true },
            modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
            Text("Install selected (${state.selected.size})")
        }
    }
    if (confirm) AlertDialog(
        onDismissRequest = { confirm = false },
        title = { Text("Install selected modules") },
        text = { Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(state.active)
            state.packages.filter { it.key in state.selected }.forEach { Text("${it.name} · ${it.version}") }
            Text("GXR replacements stay DISABLED. No automatic restart. Other modules use their standard installer.")
        } },
        confirmButton = { TextButton(onClick = { confirm = false; model.installSelected(installed) }) { Text("Install") } },
        dismissButton = { TextButton(onClick = { confirm = false }) { Text("Cancel") } },
    )
}
