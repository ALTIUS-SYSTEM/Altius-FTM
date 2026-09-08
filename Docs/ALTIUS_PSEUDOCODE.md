# Altius FTM — Pseudocode

> **Product:** Altius FTM (Fleet & Transport Management)
> **Date:** 2026-09-07  

---

## Table of Contents

1. [Authentication Flow](#1-authentication-flow)
2. [OTP Verification Flow](#2-otp-verification-flow)
3. [Organization Switch Flow](#3-organization-switch-flow)
4. [Hub Selection Flow](#4-hub-selection-flow)
5. [Check-In Flow](#5-check-in-flow)
6. [Check-Out Flow](#6-check-out-flow)
7. [Task List Loading Flow](#7-task-list-loading-flow)
8. [Task Execution Engine](#8-task-execution-engine)
9. [Component Render Engine](#9-component-render-engine)
10. [Task Sync Flow](#10-task-sync-flow)
11. [Data Sync Engine](#11-data-sync-engine)
12. [Page Webhook Processing](#12-page-webhook-processing)
13. [Route Optimization Flow](#13-route-optimization-flow)
14. [QR Login Flow](#14-qr-login-flow)
15. [Sync Failure Report Generation](#15-sync-failure-report-generation)
16. [Mock GPS Detection Flow](#16-mock-gps-detection-flow)
17. [App Version Check Flow](#17-app-version-check-flow)
18. [Location History Recording](#18-location-history-recording)
19. [Organization Switch with Data Clearing](#19-organization-switch-with-data-clearing)

---

## 1. Authentication Flow

```
function login(email, password):
    // Step 1: Call login API
    response = POST /auth {
        email: email,
        password: hash(password)
    }
    
    // Step 2: Handle MFA if required
    if response.mfa_required:
        if response.mfa_type == "otp":
            return navigateToOTP(response.mfa_token)
        elif response.mfa_type == "totp":
            return navigateToCloudAuthenticator(response.mfa_token)
    
    // Step 3: Handle password expiry
    if response.password_expired:
        return navigateToChangePassword(response.change_password_token)
    
    // Step 4: Save session
    saveTokenToLocal(response.token)
    saveUserDataToLocal(response.user)
    saveOrgIdToLocal(response.user.org_id)
    
    // Step 5: Register FCM token
    fcmToken = getFCMToken()
    if fcmToken:
        registerDeviceToken(response.user.id, fcmToken)
        saveFcmTokenToLocal(fcmToken)
    
    // Step 6: Sync initial data
    syncVersionData()
    
    // Step 7: Check if password expired
    if isPasswordExpired(response.user):
        navigateToChangePassword()
        return
    
    // Step 8: Navigate based on org/hub status
    orgId = getOrgIdFromLocal()
    hubId = getHubIdFromLocal()
    
    if !orgId:
        navigateToOrganizations()
    elif !hubId:
        navigateToHubChooser()
    else:
        navigateToMainMenu()
```

## 2. OTP Verification Flow

```
function verifyOTP(mfaToken, code):
    response = POST /otp/verify {
        mfa_token: mfaToken,
        code: code
    }
    
    if response.verified:
        saveTokenToLocal(response.token)
        saveUserDataToLocal(response.user)
        registerDeviceToken(response.user.id, getFCMToken())
        syncVersionData()
        navigateToMainMenu()
    else:
        if response.attempts_remaining > 0:
            showError("Invalid code. Attempts remaining: " + response.attempts_remaining)
            // User can retry or resend
        else:
            showError("Invalid attempts reached. Please tap 'Resend' to re-verify.")
            // Block further attempts, require resend

function sendOTP(recipient, serviceSender):
    response = POST /otp/send {
        recipient: recipient,
        service_sender: serviceSender
    }
    if response.success:
        showInfo("OTP sent to " + recipient)
    else:
        showError("Failed to send OTP")
```

## 3. Organization Switch Flow

```
function switchOrganization(newOrgId):
    // Step 1: Call switch API
    showLoading("Changing Organization...")
    response = POST /organization/switch {
        organization_id: newOrgId
    }
    
    if response.error:
        showError("Failed to change organization")
        return
    
    // Step 2: Save new token and org
    saveTokenToLocal(response.token)
    saveOrgIdToLocal(newOrgId)
    
    // Step 3: Clear all org-specific local data
    removeLocalTaskData()
    clearFlowData()
    clearCurrencyData()
    clearOtpCache()
    clearEntityData()
    
    // Step 4: Fetch new org configuration
    orgConfig = getOrgConfiguration(newOrgId)
    saveOrgConfigurationToLocal(orgConfig)
    
    // Step 5: Re-sync all data for new org
    syncVersionData()
    syncCurrency()
    syncEntityData()
    syncHubs()
    
    // Step 6: Navigate to hub chooser
    navigateToHubChooser()
```

## 4. Hub Selection Flow

```
function selectHub(hubId):
    // Step 1: Save selected hub
    saveCurrentHub(hubId)
    
    // Step 2: Sync hub permissions
    permissions = syncHubPermissionRemote(hubId)
    savePermissionsToLocal(permissions)
    
    // Step 3: Check check-in permission
    canCheckIn = getPermissionCheckIn()
    
    // Step 4: Navigate to main menu
    navigateToMainMenu()
    
    // Step 5: If user was previously checked in, restore state
    checkInTime = getCheckInTimeFromLocal()
    if checkInTime && !isCheckedOut(checkInTime):
        showInfo("You are currently checked in since " + checkInTime)
```

## 5. Check-In Flow

```
function checkIn(hubId, optionalInfo):
    // Step 1: Validate permissions
    if !getPermissionCheckIn():
        showError("You don't have permission to check in")
        return
    
    // Step 2: Check GPS enabled
    if !isGPSEnabled():
        showLocationDisclosureDialog()
        return
    
    // Step 3: Get current location
    location = getCurrentLocation()
    if !location:
        showError("Cannot get location")
        return
    
    // Step 4: Validate geofence
    hub = getHub(hubId)
    distance = haversine(location.lat, location.lng, 
                         hub.geoLock.lat, hub.geoLock.lng)
    
    if distance > hub.geoLock.radius:
        showError("You are " + distance + " meters away from " + hub.name)
        return
    
    // Step 5: Save check-in data
    saveCheckInTimeToLocal(currentTimestamp())
    saveInLocationData(location)
    
    // Step 6: Sync to remote
    response = syncRemoteCheckInToLocal(hubId, location, optionalInfo)
    
    // Step 7: Start foreground service (location tracking)
    startForegroundService()
    startLocationHistoryRecording()
    
    // Step 8: Show success
    showSuccess("Start Trip is Successful", {
        hub: hub.name,
        time: currentTimestamp(),
        info: optionalInfo
    })
    
    // Step 9: Navigate to task list
    navigateToTaskList()
```

## 6. Check-Out Flow

```
function checkOut(hubId):
    // Step 1: Get current location
    location = getCurrentLocation()
    
    // Step 2: Save check-out data
    saveOutLocationData(location)
    
    // Step 3: Sync to remote
    response = syncCheckOut(hubId, location)
    
    // Step 4: Stop foreground service
    stopForegroundService()
    stopLocationHistoryRecording()
    
    // Step 5: Sync remaining location history
    syncLocationHistoryBulk()
    
    // Step 6: Show success
    showSuccess("End Trip is Successful", {
        hub: hub.name,
        time: currentTimestamp(),
        duration: calculateDuration(checkInTime, currentTime)
    })
    
    // Step 7: Check if all tasks are done
    ongoingTasks = getOngoingTasks()
    if ongoingTasks.length == 0:
        showInfo("All tasks completed. Have a good day!")
    else:
        showWarning("You have " + ongoingTasks.length + " incomplete tasks")
```

## 7. Task List Loading Flow

```
function loadTaskList():
    // Step 1: Try local first (offline-first)
    localTasks = getTaskListLocal()
    
    if localTasks.length > 0:
        displayTaskList(localTasks)
        // Show sync status indicator
        unsyncedCount = countUnsynced(localTasks)
        if unsyncedCount > 0:
            showSyncBadge(unsyncedCount)
    
    // Step 2: Fetch from remote in background
    if isOnline():
        remoteTasks = getTaskListRemote()
        
        // Step 3: Merge remote with local
        for remoteTask in remoteTasks:
            localTask = findLocalTask(remoteTask.id)
            if !localTask:
                // New task from server
                saveTaskToLocal(remoteTask)
            elif localTask.status == "done" && !localTask.is_synced:
                // Local has unsynced changes, keep local
                continue
            elif remoteTask.updated_at > localTask.updated_at:
                // Remote is newer, update local
                updateLocalTask(remoteTask)
        
        // Step 4: Check for removed tasks
        for localTask in localTasks:
            if !findRemoteTask(localTask.id) && localTask.is_synced:
                markTaskAsRemoved(localTask)
        
        // Step 5: Refresh display
        displayTaskList(getTaskListLocal())
    
    // Step 6: Handle errors
    if !isOnline():
        showInfo("No internet connection. Showing cached tasks.")
```

## 8. Task Execution Engine

```
function executeTask(taskId):
    // Step 1: Load task and flow
    task = getDetailTask(taskId)
    flow = getDetailFlow(task.flow_id)
    
    // Step 2: Validate task step order
    stepOrder = validateTaskStepOrder(task)
    if !stepOrder.valid:
        showError("Cannot do this task. Finish the previous task activity first.")
        return
    
    // Step 3: Get hub settings
    hubSettings = getHubSettings(task.hub_id)
    
    // Step 4: Get org configuration
    orgConfig = getOrgConfiguration(getOrgIdFromLocal())
    
    // Step 5: Render pages sequentially
    currentPageIndex = task.last_done_order || 0
    
    for i = currentPageIndex; i < flow.pages.length; i++:
        page = flow.pages[i]
        
        // Render page
        renderPageTitle(page.title)
        pageData = {}
        allComponentsValid = true
        
        for component in page.components:
            // Render component
            result = renderComponent(component, pageData)
            
            // Validate
            if component.required && !result.filled:
                showRequiredFieldsDialog()
                allComponentsValid = false
                break
            
            // Save component data
            pageData[component.id] = result.data
            
            // Special handling
            if component.type == "photo" || component.type == "video":
                // Queue media for upload
                queueMediaUpload(taskId, page.id, component.id, result.file)
        
        if !allComponentsValid:
            return  // Stay on current page
        
        // Save page data to local
        saveTaskData(taskId, page.id, pageData)
        
        // Queue page webhook if configured
        if page.webhook_url:
            addPageWebhookDataToQueue(taskId, page.id, page.webhook_url, pageData)
        
        // Update progress
        updateLastDoneOrder(taskId, i)
        
        // Navigate to next page
        if i < flow.pages.length - 1:
            navigateToNextPage(flow.pages[i + 1])
    
    // Step 6: Task completed
    updateTaskStatus(taskId, "done")
    
    // Step 7: Sync to server
    syncTaskToServer(taskId)
    uploadPendingMedia(taskId)
    processWebhookQueue(taskId)
    
    // Step 8: Show completion
    showSuccess("Task completed")
    navigateToTaskList()
```

## 9. Component Render Engine

```
function renderComponent(component, existingData):
    switch component.type:
        case "photo":
            return renderPhotoComponent(component)
        case "video":
            return renderVideoComponent(component)
        case "input":
            return renderInputComponent(component)
        case "select":
            return renderSelectComponent(component)
        case "list":
            return renderListComponent(component)
        case "bill":
            return renderBillComponent(component)
        case "scan_display":
            return renderScanDisplayComponent(component)
        case "otp":
            return renderOTPComponent(component)
        case "capture":
            return renderCaptureComponent(component)
        case "voice":
            return renderVoiceComponent(component)
        case "print":
            return renderPrintComponent(component)
        case "view":
            return renderViewComponent(component)
        case "subpage":
            return renderSubpageComponent(component)

function renderPhotoComponent(component):
    // Show photo grid view
    photos = existingData[component.id] || []
    
    // Categories dropdown if configured
    if component.has_categories:
        category = showCategoriesDropdown(component.categories)
    
    // Footer buttons: Camera, Gallery
    onCameraTap:
        navigateToCamera()
        photo = capturePhoto()
        if photo:
            // Check max limit
            if photos.length >= component.max_page:
                showError("Max " + component.max_page + " photo")
                return
            photos.push({ file: photo, category: category })
            updatePhotoGridView(photos)
    
    onGalleryTap:
        navigateToGallery()
        selectedPhotos = selectFromGallery()
        for photo in selectedPhotos:
            if photos.length >= component.max_page:
                showError("Max " + component.max_page + " photo")
                break
            photos.push({ file: photo, category: category })
        updatePhotoGridView(photos)
    
    return { filled: photos.length > 0, data: photos }

function renderVideoComponent(component):
    // Options: Camera or Gallery
    onCameraTap:
        navigateToVideoCamera()
        video = recordVideo(component.max_duration)
        if video.duration > component.max_duration:
            showError("Video too long. Maximum " + component.max_duration + " seconds.")
            return
        // Compress video
        compressed = compressVideo(video)
        return { filled: true, data: compressed }
    
    onGalleryTap:
        navigateToVideoGallery()
        video = selectVideoFromGallery()
        compressed = compressVideo(video)
        return { filled: true, data: compressed }

function renderInputComponent(component):
    switch component.input_type:
        case "text":
            value = showTextField(component.placeholder)
            validate(value, component.validation_rules)
        case "currency":
            currency = getCurrencyCode()
            value = showCurrencyField(currency)
            validate(value, { min: component.min, max: component.max })
        case "date":
            value = showDatePicker()
            validate(value, { min_date: component.min_date, max_date: component.max_date })
        case "url":
            value = showURLField()
            validateURL(value)
        case "entity_data":
            entity = showEntityDataSearch(component.data_type_id)
            value = entity.selected_item
    return { filled: value != null && value != "", data: value }

function renderBillComponent(component):
    items = []
    costs = []
    
    // Items section
    for itemConfig in component.items:
        item = {
            name: itemConfig.name,
            qty: itemConfig.initial_qty || 0,
            unit_price: itemConfig.unit_price
        }
        // Validate qty bounds
        if item.qty < itemConfig.initial_qty:
            showError("Cannot decrease below initial value")
        if item.qty > itemConfig.initial_qty:
            showError("Cannot increase above initial value")
        items.push(item)
    
    // Costs section
    for costConfig in component.costs:
        cost = {
            name: costConfig.name,
            value: showCostInput(costConfig)
        }
        costs.push(cost)
    
    // Summary
    summary = {
        total_items: items.length,
        total_cost: sum(costs.value),
        items: items,
        costs: costs
    }
    
    // Validate reason required
    for item in items:
        if !item.reason && component.reason_required:
            showError("There are data bills that do not have a reason.")
            return { filled: false }
    
    return { filled: items.length > 0, data: summary }

function renderOTPComponent(component):
    // Send OTP to recipient
    sendOTP(component.recipient, component.service_sender)
    showInfo("OTP sent to " + maskRecipient(component.recipient))
    
    // User enters code
    code = showOTPInputField(6)  // 6-digit code
    
    // Verify
    result = verifyOTP(code)
    if !result.verified:
        if result.expired:
            showError("The one-time passcode has expired.")
        else:
            showError("Invalid code. Attempts remaining: " + result.attempts_remaining)
        return { filled: false }
    
    return { filled: true, data: { verified: true, code: code } }

function renderScanDisplayComponent(component):
    onScanTap:
        navigateToBarcodeScanner()
        barcode = scanBarcode()
        // Support multiple QR/barcode
        if component.allow_multiple:
            barcodes.push(barcode)
        else:
            barcodes = [barcode]
        displayScanResult(barcode)
    return { filled: barcodes.length > 0, data: barcodes }

function renderVoiceComponent(component):
    onRecordTap:
        startRecording()
    onStopTap:
        stopRecording()
        audioFile = saveRecording()
        showPlayButton(audioFile)
    
    onRetakeTap:
        confirmDialog("Retake Voice Recording", 
            "Are you sure you want to retake? Previous recording will be deleted.")
        if confirmed:
            deleteRecording()
            startRecording()
    
    return { filled: audioFile != null, data: audioFile }

function renderSubpageComponent(component):
    // Show subpage list
    entries = existingData[component.id] || []
    
    // Subpage list content
    displaySubpageList(entries)
    
    // Add entry
    onAddTap:
        navigateToSubpageForm(component.subpage_config)
        entry = collectSubpageFormData()
        entries.push(entry)
        updateSubpageList(entries)
    
    // Validate at least one entry
    if entries.length == 0:
        showError("Please fill in at least one entry to proceed")
        return { filled: false }
    
    return { filled: true, data: entries }
```

## 10. Task Sync Flow

```
function syncTaskToServer(taskId):
    task = getTaskFromLocal(taskId)
    
    if task.is_synced:
        return  // Already synced
    
    // Step 1: Upload media first
    for media in task.pending_media:
        if media.type == "photo":
            response = uploadToS3(media.file)
            if response.success:
                media.url = response.url
                markMediaUploaded(media.id)
            else:
                generateSyncFailureReport(taskId, "upload", media)
                return  // Cannot sync task data without media
    
    // Step 2: Upload task data
    response = POST /tasks/bulk {
        tasks: [{
            id: task.id,
            status: task.status,
            data: task.data,
            media_urls: extractMediaUrls(task)
        }]
    }
    
    if response.success:
        markTaskSynced(taskId)
        showInfo("Task has been synced")
    else:
        generateSyncFailureReport(taskId, "task_data", task)
        showError("Data sync failed. Please try again.")
```

## 11. Data Sync Engine

```
function sync():
    // Step 1: Check version
    versionData = checkVersion()
    
    if versionData.flow_version_changed:
        flows = downloadFlows()
        saveFlowsToLocal(flows)
    
    if versionData.data_version_changed:
        syncEntityData()
        syncCurrencies()
        syncHubs()
    
    // Step 2: Sync FCM token
    syncFcmToken()
    
    // Step 3: Sync tasks
    pendingTasks = getUnsyncedTasks()
    for task in pendingTasks:
        syncTaskToServer(task.id)
    
    // Step 4: Process webhook queue
    processWebhookQueue()
    
    // Step 5: Sync location history
    pendingLocations = getUnsyncedLocationHistory()
    if pendingLocations.length > 0:
        POST /location-history/bulk { records: pendingLocations }
        markLocationsSynced(pendingLocations)
    
    // Step 6: Clean up
    removeUnusedFiles()
    clearFinishedWebhookProcesses()

function syncEntityData():
    // Stream sync for large entity data
    stream = streamGetEntityRemote()
    for batch in stream:
        saveEntityDataToLocal(batch)
```

## 12. Page Webhook Processing

```
function processWebhookQueue():
    queue = getQueueDataWebhookProcess()
    
    for item in queue:
        if item.status == "pending":
            // Update status to processing
            updatePageWebhookQueueData(item.id, "processing")
            
            // Send webhook
            response = POST item.webhook_url {
                task_id: item.task_id,
                page_id: item.page_id,
                data: item.data
            }
            
            if response.success:
                // Mark as done
                updatePageWebhookQueueData(item.id, "done")
            else:
                // Increment attempts
                item.attempts += 1
                if item.attempts >= MAX_ATTEMPTS:
                    updatePageWebhookQueueData(item.id, "failed")
                    generateSyncFailureReport(item.task_id, "page_webhook", item)
                else:
                    updatePageWebhookQueueData(item.id, "pending")
    
    // Clear finished processes
    clearFinishedWebhookProcesses()
```

## 13. Route Optimization Flow

```
function optimizeRoute():
    // Step 1: Get all ongoing tasks with locations
    tasks = getOngoingTasks()
    taskLocations = tasks.map(t => t.location)
    
    if taskLocations.length < 2:
        showInfo("Need at least 2 tasks to optimize route")
        return
    
    // Step 2: Confirm with user
    confirmed = showConfirmDialog(
        "This will change the delivery order to prioritize the closest locations first. Would you like to continue?"
    )
    if !confirmed:
        return
    
    // Step 3: Show loading
    showLoadingDialog("Optimizing route...")
    
    // Step 4: Call optimization API
    request = RouteOptimizationReqModel {
        locations: taskLocations,
        origin: getCurrentLocation()
    }
    
    response = getRouteOptimization(request)
    
    if response.error:
        showError("Route Optimization Failed")
        hideLoadingDialog()
        return
    
    // Step 5: Reorder tasks
    optimizedOrder = response.optimized_order
    for i = 0; i < optimizedOrder.length; i++:
        updateTaskOrder(optimizedOrder[i].task_id, i)
    
    // Step 6: Refresh task list
    refreshTaskList()
    hideLoadingDialog()
    showSuccess("Route optimized successfully")
```

## 14. QR Login Flow

```
// Mobile side: Scan QR from web app
function scanQRForLogin():
    // User navigates to Settings → Profile QR
    qrCode = scanBarcode()
    
    // QR contains a login token
    loginToken = parseQRCode(qrCode)
    
    // Send token to backend to authenticate web session
    response = loginWithToken(loginToken)
    
    if response.success:
        showSuccess("Web login successful")
    else:
        showError("Failed to login to web")

// Web side (inferred):
// 1. Web app displays QR code containing a login token
// 2. User opens mobile app → Settings → Profile QR → Scan
// 3. Mobile sends token to backend
// 4. Backend authenticates web session
// 5. Web app receives auth token and logs in
```

## 15. Sync Failure Report Generation

```
function generateSyncFailureReport(taskId, failureType, detail):
    report = {
        task_id: taskId,
        type: failureType,  // "task_data", "upload", "page_webhook", etc.
        detail: detail,
        created_at: currentTimestamp()
    }
    
    saveSyncFailureReport(report)
    
    // Categorize
    switch failureType:
        case "task_data":
            category = "Task Data"
            message = "These tasks couldn't be synced because their data is invalid."
        case "upload":
            category = "Media Upload"
            message = "These tasks couldn't be synced because their media failed to upload."
        case "page_webhook":
            category = "Page Webhook"
            message = "These tasks couldn't be synced due to failed automated action."
        case "reassigned":
            category = "Reassigned"
            message = "These tasks are no longer assigned to you."
        case "removed":
            category = "Removed"
            message = "These tasks are no longer available."
        case "currency":
            category = "Currency"
            message = "These failures happened when trying to sync the currency."
        case "flow":
            category = "Flow"
            message = "These failures happened when trying to sync the flow."
        case "location_history":
            category = "Location History"
            message = "These failures happened when trying to sync the location history."
    
    // Show in sync failure sheet
    showSyncFailureSheet(category, message, report)

function showSyncFailureSheet():
    // Group by source
    groups = groupByFailureType(getAllSyncFailureReports())
    
    for group in groups:
        renderSyncFailureSourceGroup(group)
        for task in group.tasks:
            renderSyncFailureTaskCard(task)
    
    // Copy hint
    showCopyHint("You can tap Copy to copy the details and report them to your admin.")
    
    // Actions
    onCopyAll:
        copyToClipboard(formatReports(reports))
    onRetrySync:
        sync()
```

## 16. Mock GPS Detection Flow

```
function checkMockGPS():
    // Step 1: Native library check
    isMockEnabled = libtoolChecker.checkMockLocation()
    
    if isMockEnabled:
        navigateToMockGpsAppDetectedPage()
        blockAppUsage()
        return
    
    // Step 2: Check for installed mock GPS apps
    mockApps = libtoolChecker.detectMockApps()
    
    if mockApps.length > 0:
        navigateToMockGpsAppDetectedPage(mockApps)
        blockAppUsage()
        return
    
    // Step 3: Check location accuracy
    location = getCurrentLocation()
    if location.is_from_mock_provider:
        navigateToMockGpsAppDetectedPage()
        blockAppUsage()
        return
    
    // All checks passed
    allowAppUsage()
```

## 17. App Version Check Flow

```
function checkAppVersion():
    // Step 1: Get version data from backend
    versionData = GET /version/
    
    // Step 2: Check if update is required (below minimum)
    if currentVersion < versionData.min_version:
        // Force update
        showForceUpdateScreen(
            "New Version Available",
            "Please update to version " + versionData.latest_version
        )
        // Link to Play Store
        openPlayStore()
        return
    
    // Step 3: Check if optional update available
    if currentVersion < versionData.latest_version:
        if !getAppUpdateAvailableFlag():
            showUpdateDialog(
                "New Version Available",
                "An update is available. Would you like to update?"
            )
            saveAppUpdateAvailableFlag(true)
    
    // Step 4: Check password expiry
    if isPasswordExpired(getUserData()):
        navigateToChangePassword()
        showInfo("Your password has expired, please change your password to keep your account secure.")
```

## 18. Location History Recording

```
function startLocationHistoryRecording():
    // Start foreground service for persistent location
    startForegroundService(FOREGROUND_SERVICE_LOCATION)
    
    // Register location updates
    locationCallback = (location) => {
        record = {
            lat: location.lat,
            lng: location.lng,
            accuracy: location.accuracy,
            timestamp: currentTimestamp(),
            connection_state: getNetworkState(),
            is_synced: false
        }
        saveLocationHistoryLocal(record)
    }
    
    requestLocationUpdates(
        interval: LOCATION_UPDATE_INTERVAL,
        callback: locationCallback
    )

function stopLocationHistoryRecording():
    stopLocationUpdates()
    stopForegroundService()
    
    // Sync remaining location history
    syncLocationHistoryBulk()

function syncLocationHistoryBulk():
    pendingRecords = getUnsyncedLocationHistory()
    
    if pendingRecords.length == 0:
        return
    
    // Get routing result ID (for optimized upload)
    routingResultId = getRoutingResultId()
    
    response = POST /location-history/bulk {
        records: pendingRecords,
        routing_result_id: routingResultId
    }
    
    if response.success:
        markLocationsSynced(pendingRecords)
    else:
        generateSyncFailureReport(null, "location_history", pendingRecords)
```

## 19. Organization Switch with Data Clearing

```
function switchOrganizationWithClearing(newOrgId):
    showLoading("Changing Organization...")
    
    // Step 1: Call API to switch
    response = POST /organization/switch { organization_id: newOrgId }
    
    if !response.success:
        showError("Failed to change organization")
        hideLoading()
        return
    
    // Step 2: Save new token
    saveTokenToLocal(response.token)
    saveOrgIdToLocal(newOrgId)
    
    // Step 3: Clear ALL org-specific local data
    try:
        removeLocalTaskData()        // Delete all tasks
    catch e:
        log("Failed to remove task data: " + e)
    
    try:
        clearFlowData()              // Delete flow definitions
    catch e:
        log("Failed to clear flow data: " + e)
    
    try:
        clearCurrencyData()          // Delete currency master
    catch e:
        log("Failed to clear currency data: " + e)
    
    try:
        clearOtpCache()              // Clear OTP cache
    catch e:
        log("Failed to clear OTP cache: " + e)
    
    try:
        clearEntityData()            // Clear entity/master data
    catch e:
        log("Failed to clear entity data: " + e)
    
    try:
        removeUnusedFiles()          // Clean up media files
    catch e:
        log("Failed to remove unused files: " + e)
    
    // Step 4: Fetch new org configuration
    orgConfig = getOrgConfiguration(newOrgId)
    saveOrgConfigurationToLocal(orgConfig)
    
    // Step 5: Save Avian flag if applicable
    saveIsAvianToLocal(orgConfig.is_avian)
    
    // Step 6: Re-sync all reference data
    syncVersionData()
    syncCurrency()
    syncEntityData()
    syncHubs()
    
    // Step 7: Clear hub selection (force re-selection)
    clearHubIdFromLocal()
    
    hideLoading()
    
    // Step 8: Navigate to hub chooser
    navigateToHubChooser()
```

---

*End of Pseudocode*
