// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'dart_types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$StreamEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'StreamEvent()';
}


}

/// @nodoc
class $StreamEventCopyWith<$Res>  {
$StreamEventCopyWith(StreamEvent _, $Res Function(StreamEvent) __);
}


/// Adds pattern-matching-related methods to [StreamEvent].
extension StreamEventPatterns on StreamEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( StreamEvent_Error value)?  error,TResult Function( StreamEvent_CurrentTime value)?  currentTime,TResult Function( StreamEvent_OriginVideoSize value)?  originVideoSize,TResult Function( StreamEvent_WscRtpSessionMode value)?  wscRtpSessionMode,TResult Function( StreamEvent_WscRtpPlayback value)?  wscRtpPlayback,required TResult orElse(),}){
final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that);case StreamEvent_WscRtpPlayback() when wscRtpPlayback != null:
return wscRtpPlayback(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( StreamEvent_Error value)  error,required TResult Function( StreamEvent_CurrentTime value)  currentTime,required TResult Function( StreamEvent_OriginVideoSize value)  originVideoSize,required TResult Function( StreamEvent_WscRtpSessionMode value)  wscRtpSessionMode,required TResult Function( StreamEvent_WscRtpPlayback value)  wscRtpPlayback,}){
final _that = this;
switch (_that) {
case StreamEvent_Error():
return error(_that);case StreamEvent_CurrentTime():
return currentTime(_that);case StreamEvent_OriginVideoSize():
return originVideoSize(_that);case StreamEvent_WscRtpSessionMode():
return wscRtpSessionMode(_that);case StreamEvent_WscRtpPlayback():
return wscRtpPlayback(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( StreamEvent_Error value)?  error,TResult? Function( StreamEvent_CurrentTime value)?  currentTime,TResult? Function( StreamEvent_OriginVideoSize value)?  originVideoSize,TResult? Function( StreamEvent_WscRtpSessionMode value)?  wscRtpSessionMode,TResult? Function( StreamEvent_WscRtpPlayback value)?  wscRtpPlayback,}){
final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that);case StreamEvent_WscRtpPlayback() when wscRtpPlayback != null:
return wscRtpPlayback(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  error,TResult Function( PlatformInt64 field0)?  currentTime,TResult Function( BigInt width,  BigInt height)?  originVideoSize,TResult Function( WscRtpMode field0)?  wscRtpSessionMode,TResult Function( WscRtpPlaybackEvent field0)?  wscRtpPlayback,required TResult orElse(),}) {final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that.field0);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that.field0);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that.width,_that.height);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that.field0);case StreamEvent_WscRtpPlayback() when wscRtpPlayback != null:
return wscRtpPlayback(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  error,required TResult Function( PlatformInt64 field0)  currentTime,required TResult Function( BigInt width,  BigInt height)  originVideoSize,required TResult Function( WscRtpMode field0)  wscRtpSessionMode,required TResult Function( WscRtpPlaybackEvent field0)  wscRtpPlayback,}) {final _that = this;
switch (_that) {
case StreamEvent_Error():
return error(_that.field0);case StreamEvent_CurrentTime():
return currentTime(_that.field0);case StreamEvent_OriginVideoSize():
return originVideoSize(_that.width,_that.height);case StreamEvent_WscRtpSessionMode():
return wscRtpSessionMode(_that.field0);case StreamEvent_WscRtpPlayback():
return wscRtpPlayback(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  error,TResult? Function( PlatformInt64 field0)?  currentTime,TResult? Function( BigInt width,  BigInt height)?  originVideoSize,TResult? Function( WscRtpMode field0)?  wscRtpSessionMode,TResult? Function( WscRtpPlaybackEvent field0)?  wscRtpPlayback,}) {final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that.field0);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that.field0);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that.width,_that.height);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that.field0);case StreamEvent_WscRtpPlayback() when wscRtpPlayback != null:
return wscRtpPlayback(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class StreamEvent_Error extends StreamEvent {
  const StreamEvent_Error(this.field0): super._();
  

 final  String field0;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamEvent_ErrorCopyWith<StreamEvent_Error> get copyWith => _$StreamEvent_ErrorCopyWithImpl<StreamEvent_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent_Error&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamEvent.error(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamEvent_ErrorCopyWith<$Res> implements $StreamEventCopyWith<$Res> {
  factory $StreamEvent_ErrorCopyWith(StreamEvent_Error value, $Res Function(StreamEvent_Error) _then) = _$StreamEvent_ErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$StreamEvent_ErrorCopyWithImpl<$Res>
    implements $StreamEvent_ErrorCopyWith<$Res> {
  _$StreamEvent_ErrorCopyWithImpl(this._self, this._then);

  final StreamEvent_Error _self;
  final $Res Function(StreamEvent_Error) _then;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamEvent_Error(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class StreamEvent_CurrentTime extends StreamEvent {
  const StreamEvent_CurrentTime(this.field0): super._();
  

 final  PlatformInt64 field0;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamEvent_CurrentTimeCopyWith<StreamEvent_CurrentTime> get copyWith => _$StreamEvent_CurrentTimeCopyWithImpl<StreamEvent_CurrentTime>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent_CurrentTime&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamEvent.currentTime(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamEvent_CurrentTimeCopyWith<$Res> implements $StreamEventCopyWith<$Res> {
  factory $StreamEvent_CurrentTimeCopyWith(StreamEvent_CurrentTime value, $Res Function(StreamEvent_CurrentTime) _then) = _$StreamEvent_CurrentTimeCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 field0
});




}
/// @nodoc
class _$StreamEvent_CurrentTimeCopyWithImpl<$Res>
    implements $StreamEvent_CurrentTimeCopyWith<$Res> {
  _$StreamEvent_CurrentTimeCopyWithImpl(this._self, this._then);

  final StreamEvent_CurrentTime _self;
  final $Res Function(StreamEvent_CurrentTime) _then;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamEvent_CurrentTime(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class StreamEvent_OriginVideoSize extends StreamEvent {
  const StreamEvent_OriginVideoSize({required this.width, required this.height}): super._();
  

 final  BigInt width;
 final  BigInt height;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamEvent_OriginVideoSizeCopyWith<StreamEvent_OriginVideoSize> get copyWith => _$StreamEvent_OriginVideoSizeCopyWithImpl<StreamEvent_OriginVideoSize>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent_OriginVideoSize&&(identical(other.width, width) || other.width == width)&&(identical(other.height, height) || other.height == height));
}


@override
int get hashCode => Object.hash(runtimeType,width,height);

@override
String toString() {
  return 'StreamEvent.originVideoSize(width: $width, height: $height)';
}


}

/// @nodoc
abstract mixin class $StreamEvent_OriginVideoSizeCopyWith<$Res> implements $StreamEventCopyWith<$Res> {
  factory $StreamEvent_OriginVideoSizeCopyWith(StreamEvent_OriginVideoSize value, $Res Function(StreamEvent_OriginVideoSize) _then) = _$StreamEvent_OriginVideoSizeCopyWithImpl;
@useResult
$Res call({
 BigInt width, BigInt height
});




}
/// @nodoc
class _$StreamEvent_OriginVideoSizeCopyWithImpl<$Res>
    implements $StreamEvent_OriginVideoSizeCopyWith<$Res> {
  _$StreamEvent_OriginVideoSizeCopyWithImpl(this._self, this._then);

  final StreamEvent_OriginVideoSize _self;
  final $Res Function(StreamEvent_OriginVideoSize) _then;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? width = null,Object? height = null,}) {
  return _then(StreamEvent_OriginVideoSize(
width: null == width ? _self.width : width // ignore: cast_nullable_to_non_nullable
as BigInt,height: null == height ? _self.height : height // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class StreamEvent_WscRtpSessionMode extends StreamEvent {
  const StreamEvent_WscRtpSessionMode(this.field0): super._();
  

 final  WscRtpMode field0;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamEvent_WscRtpSessionModeCopyWith<StreamEvent_WscRtpSessionMode> get copyWith => _$StreamEvent_WscRtpSessionModeCopyWithImpl<StreamEvent_WscRtpSessionMode>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent_WscRtpSessionMode&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamEvent.wscRtpSessionMode(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamEvent_WscRtpSessionModeCopyWith<$Res> implements $StreamEventCopyWith<$Res> {
  factory $StreamEvent_WscRtpSessionModeCopyWith(StreamEvent_WscRtpSessionMode value, $Res Function(StreamEvent_WscRtpSessionMode) _then) = _$StreamEvent_WscRtpSessionModeCopyWithImpl;
@useResult
$Res call({
 WscRtpMode field0
});


$WscRtpModeCopyWith<$Res> get field0;

}
/// @nodoc
class _$StreamEvent_WscRtpSessionModeCopyWithImpl<$Res>
    implements $StreamEvent_WscRtpSessionModeCopyWith<$Res> {
  _$StreamEvent_WscRtpSessionModeCopyWithImpl(this._self, this._then);

  final StreamEvent_WscRtpSessionMode _self;
  final $Res Function(StreamEvent_WscRtpSessionMode) _then;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamEvent_WscRtpSessionMode(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as WscRtpMode,
  ));
}

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$WscRtpModeCopyWith<$Res> get field0 {
  
  return $WscRtpModeCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc


class StreamEvent_WscRtpPlayback extends StreamEvent {
  const StreamEvent_WscRtpPlayback(this.field0): super._();
  

 final  WscRtpPlaybackEvent field0;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamEvent_WscRtpPlaybackCopyWith<StreamEvent_WscRtpPlayback> get copyWith => _$StreamEvent_WscRtpPlaybackCopyWithImpl<StreamEvent_WscRtpPlayback>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent_WscRtpPlayback&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamEvent.wscRtpPlayback(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamEvent_WscRtpPlaybackCopyWith<$Res> implements $StreamEventCopyWith<$Res> {
  factory $StreamEvent_WscRtpPlaybackCopyWith(StreamEvent_WscRtpPlayback value, $Res Function(StreamEvent_WscRtpPlayback) _then) = _$StreamEvent_WscRtpPlaybackCopyWithImpl;
@useResult
$Res call({
 WscRtpPlaybackEvent field0
});


$WscRtpPlaybackEventCopyWith<$Res> get field0;

}
/// @nodoc
class _$StreamEvent_WscRtpPlaybackCopyWithImpl<$Res>
    implements $StreamEvent_WscRtpPlaybackCopyWith<$Res> {
  _$StreamEvent_WscRtpPlaybackCopyWithImpl(this._self, this._then);

  final StreamEvent_WscRtpPlayback _self;
  final $Res Function(StreamEvent_WscRtpPlayback) _then;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamEvent_WscRtpPlayback(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as WscRtpPlaybackEvent,
  ));
}

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$WscRtpPlaybackEventCopyWith<$Res> get field0 {
  
  return $WscRtpPlaybackEventCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc
mixin _$StreamMessage {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamMessage&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'StreamMessage(field0: $field0)';
}


}

/// @nodoc
class $StreamMessageCopyWith<$Res>  {
$StreamMessageCopyWith(StreamMessage _, $Res Function(StreamMessage) __);
}


/// Adds pattern-matching-related methods to [StreamMessage].
extension StreamMessagePatterns on StreamMessage {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( StreamMessage_State value)?  state,TResult Function( StreamMessage_Event value)?  event,required TResult orElse(),}){
final _that = this;
switch (_that) {
case StreamMessage_State() when state != null:
return state(_that);case StreamMessage_Event() when event != null:
return event(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( StreamMessage_State value)  state,required TResult Function( StreamMessage_Event value)  event,}){
final _that = this;
switch (_that) {
case StreamMessage_State():
return state(_that);case StreamMessage_Event():
return event(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( StreamMessage_State value)?  state,TResult? Function( StreamMessage_Event value)?  event,}){
final _that = this;
switch (_that) {
case StreamMessage_State() when state != null:
return state(_that);case StreamMessage_Event() when event != null:
return event(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( StreamState field0)?  state,TResult Function( StreamEvent field0)?  event,required TResult orElse(),}) {final _that = this;
switch (_that) {
case StreamMessage_State() when state != null:
return state(_that.field0);case StreamMessage_Event() when event != null:
return event(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( StreamState field0)  state,required TResult Function( StreamEvent field0)  event,}) {final _that = this;
switch (_that) {
case StreamMessage_State():
return state(_that.field0);case StreamMessage_Event():
return event(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( StreamState field0)?  state,TResult? Function( StreamEvent field0)?  event,}) {final _that = this;
switch (_that) {
case StreamMessage_State() when state != null:
return state(_that.field0);case StreamMessage_Event() when event != null:
return event(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class StreamMessage_State extends StreamMessage {
  const StreamMessage_State(this.field0): super._();
  

@override final  StreamState field0;

/// Create a copy of StreamMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamMessage_StateCopyWith<StreamMessage_State> get copyWith => _$StreamMessage_StateCopyWithImpl<StreamMessage_State>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamMessage_State&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamMessage.state(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamMessage_StateCopyWith<$Res> implements $StreamMessageCopyWith<$Res> {
  factory $StreamMessage_StateCopyWith(StreamMessage_State value, $Res Function(StreamMessage_State) _then) = _$StreamMessage_StateCopyWithImpl;
@useResult
$Res call({
 StreamState field0
});


$StreamStateCopyWith<$Res> get field0;

}
/// @nodoc
class _$StreamMessage_StateCopyWithImpl<$Res>
    implements $StreamMessage_StateCopyWith<$Res> {
  _$StreamMessage_StateCopyWithImpl(this._self, this._then);

  final StreamMessage_State _self;
  final $Res Function(StreamMessage_State) _then;

/// Create a copy of StreamMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamMessage_State(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as StreamState,
  ));
}

/// Create a copy of StreamMessage
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$StreamStateCopyWith<$Res> get field0 {
  
  return $StreamStateCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc


class StreamMessage_Event extends StreamMessage {
  const StreamMessage_Event(this.field0): super._();
  

@override final  StreamEvent field0;

/// Create a copy of StreamMessage
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamMessage_EventCopyWith<StreamMessage_Event> get copyWith => _$StreamMessage_EventCopyWithImpl<StreamMessage_Event>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamMessage_Event&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamMessage.event(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamMessage_EventCopyWith<$Res> implements $StreamMessageCopyWith<$Res> {
  factory $StreamMessage_EventCopyWith(StreamMessage_Event value, $Res Function(StreamMessage_Event) _then) = _$StreamMessage_EventCopyWithImpl;
@useResult
$Res call({
 StreamEvent field0
});


$StreamEventCopyWith<$Res> get field0;

}
/// @nodoc
class _$StreamMessage_EventCopyWithImpl<$Res>
    implements $StreamMessage_EventCopyWith<$Res> {
  _$StreamMessage_EventCopyWithImpl(this._self, this._then);

  final StreamMessage_Event _self;
  final $Res Function(StreamMessage_Event) _then;

/// Create a copy of StreamMessage
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamMessage_Event(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as StreamEvent,
  ));
}

/// Create a copy of StreamMessage
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$StreamEventCopyWith<$Res> get field0 {
  
  return $StreamEventCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc
mixin _$StreamState {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamState);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'StreamState()';
}


}

/// @nodoc
class $StreamStateCopyWith<$Res>  {
$StreamStateCopyWith(StreamState _, $Res Function(StreamState) __);
}


/// Adds pattern-matching-related methods to [StreamState].
extension StreamStatePatterns on StreamState {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( StreamState_Error value)?  error,TResult Function( StreamState_Loading value)?  loading,TResult Function( StreamState_Playing value)?  playing,TResult Function( StreamState_Stopped value)?  stopped,required TResult orElse(),}){
final _that = this;
switch (_that) {
case StreamState_Error() when error != null:
return error(_that);case StreamState_Loading() when loading != null:
return loading(_that);case StreamState_Playing() when playing != null:
return playing(_that);case StreamState_Stopped() when stopped != null:
return stopped(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( StreamState_Error value)  error,required TResult Function( StreamState_Loading value)  loading,required TResult Function( StreamState_Playing value)  playing,required TResult Function( StreamState_Stopped value)  stopped,}){
final _that = this;
switch (_that) {
case StreamState_Error():
return error(_that);case StreamState_Loading():
return loading(_that);case StreamState_Playing():
return playing(_that);case StreamState_Stopped():
return stopped(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( StreamState_Error value)?  error,TResult? Function( StreamState_Loading value)?  loading,TResult? Function( StreamState_Playing value)?  playing,TResult? Function( StreamState_Stopped value)?  stopped,}){
final _that = this;
switch (_that) {
case StreamState_Error() when error != null:
return error(_that);case StreamState_Loading() when loading != null:
return loading(_that);case StreamState_Playing() when playing != null:
return playing(_that);case StreamState_Stopped() when stopped != null:
return stopped(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  error,TResult Function()?  loading,TResult Function( PlatformInt64 textureId,  bool seekable)?  playing,TResult Function()?  stopped,required TResult orElse(),}) {final _that = this;
switch (_that) {
case StreamState_Error() when error != null:
return error(_that.field0);case StreamState_Loading() when loading != null:
return loading();case StreamState_Playing() when playing != null:
return playing(_that.textureId,_that.seekable);case StreamState_Stopped() when stopped != null:
return stopped();case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  error,required TResult Function()  loading,required TResult Function( PlatformInt64 textureId,  bool seekable)  playing,required TResult Function()  stopped,}) {final _that = this;
switch (_that) {
case StreamState_Error():
return error(_that.field0);case StreamState_Loading():
return loading();case StreamState_Playing():
return playing(_that.textureId,_that.seekable);case StreamState_Stopped():
return stopped();}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  error,TResult? Function()?  loading,TResult? Function( PlatformInt64 textureId,  bool seekable)?  playing,TResult? Function()?  stopped,}) {final _that = this;
switch (_that) {
case StreamState_Error() when error != null:
return error(_that.field0);case StreamState_Loading() when loading != null:
return loading();case StreamState_Playing() when playing != null:
return playing(_that.textureId,_that.seekable);case StreamState_Stopped() when stopped != null:
return stopped();case _:
  return null;

}
}

}

/// @nodoc


class StreamState_Error extends StreamState {
  const StreamState_Error(this.field0): super._();
  

 final  String field0;

/// Create a copy of StreamState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamState_ErrorCopyWith<StreamState_Error> get copyWith => _$StreamState_ErrorCopyWithImpl<StreamState_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamState_Error&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamState.error(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamState_ErrorCopyWith<$Res> implements $StreamStateCopyWith<$Res> {
  factory $StreamState_ErrorCopyWith(StreamState_Error value, $Res Function(StreamState_Error) _then) = _$StreamState_ErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$StreamState_ErrorCopyWithImpl<$Res>
    implements $StreamState_ErrorCopyWith<$Res> {
  _$StreamState_ErrorCopyWithImpl(this._self, this._then);

  final StreamState_Error _self;
  final $Res Function(StreamState_Error) _then;

/// Create a copy of StreamState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamState_Error(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class StreamState_Loading extends StreamState {
  const StreamState_Loading(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamState_Loading);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'StreamState.loading()';
}


}




/// @nodoc


class StreamState_Playing extends StreamState {
  const StreamState_Playing({required this.textureId, required this.seekable}): super._();
  

 final  PlatformInt64 textureId;
 final  bool seekable;

/// Create a copy of StreamState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamState_PlayingCopyWith<StreamState_Playing> get copyWith => _$StreamState_PlayingCopyWithImpl<StreamState_Playing>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamState_Playing&&(identical(other.textureId, textureId) || other.textureId == textureId)&&(identical(other.seekable, seekable) || other.seekable == seekable));
}


@override
int get hashCode => Object.hash(runtimeType,textureId,seekable);

@override
String toString() {
  return 'StreamState.playing(textureId: $textureId, seekable: $seekable)';
}


}

/// @nodoc
abstract mixin class $StreamState_PlayingCopyWith<$Res> implements $StreamStateCopyWith<$Res> {
  factory $StreamState_PlayingCopyWith(StreamState_Playing value, $Res Function(StreamState_Playing) _then) = _$StreamState_PlayingCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 textureId, bool seekable
});




}
/// @nodoc
class _$StreamState_PlayingCopyWithImpl<$Res>
    implements $StreamState_PlayingCopyWith<$Res> {
  _$StreamState_PlayingCopyWithImpl(this._self, this._then);

  final StreamState_Playing _self;
  final $Res Function(StreamState_Playing) _then;

/// Create a copy of StreamState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? textureId = null,Object? seekable = null,}) {
  return _then(StreamState_Playing(
textureId: null == textureId ? _self.textureId : textureId // ignore: cast_nullable_to_non_nullable
as PlatformInt64,seekable: null == seekable ? _self.seekable : seekable // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class StreamState_Stopped extends StreamState {
  const StreamState_Stopped(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamState_Stopped);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'StreamState.stopped()';
}


}




/// @nodoc
mixin _$WscRtpMode {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpMode);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WscRtpMode()';
}


}

/// @nodoc
class $WscRtpModeCopyWith<$Res>  {
$WscRtpModeCopyWith(WscRtpMode _, $Res Function(WscRtpMode) __);
}


/// Adds pattern-matching-related methods to [WscRtpMode].
extension WscRtpModePatterns on WscRtpMode {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( WscRtpMode_Live value)?  live,TResult Function( WscRtpMode_Dvr value)?  dvr,required TResult orElse(),}){
final _that = this;
switch (_that) {
case WscRtpMode_Live() when live != null:
return live(_that);case WscRtpMode_Dvr() when dvr != null:
return dvr(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( WscRtpMode_Live value)  live,required TResult Function( WscRtpMode_Dvr value)  dvr,}){
final _that = this;
switch (_that) {
case WscRtpMode_Live():
return live(_that);case WscRtpMode_Dvr():
return dvr(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( WscRtpMode_Live value)?  live,TResult? Function( WscRtpMode_Dvr value)?  dvr,}){
final _that = this;
switch (_that) {
case WscRtpMode_Live() when live != null:
return live(_that);case WscRtpMode_Dvr() when dvr != null:
return dvr(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  live,TResult Function( PlatformInt64 currentTimeMs,  double speed)?  dvr,required TResult orElse(),}) {final _that = this;
switch (_that) {
case WscRtpMode_Live() when live != null:
return live();case WscRtpMode_Dvr() when dvr != null:
return dvr(_that.currentTimeMs,_that.speed);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  live,required TResult Function( PlatformInt64 currentTimeMs,  double speed)  dvr,}) {final _that = this;
switch (_that) {
case WscRtpMode_Live():
return live();case WscRtpMode_Dvr():
return dvr(_that.currentTimeMs,_that.speed);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  live,TResult? Function( PlatformInt64 currentTimeMs,  double speed)?  dvr,}) {final _that = this;
switch (_that) {
case WscRtpMode_Live() when live != null:
return live();case WscRtpMode_Dvr() when dvr != null:
return dvr(_that.currentTimeMs,_that.speed);case _:
  return null;

}
}

}

/// @nodoc


class WscRtpMode_Live extends WscRtpMode {
  const WscRtpMode_Live(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpMode_Live);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WscRtpMode.live()';
}


}




/// @nodoc


class WscRtpMode_Dvr extends WscRtpMode {
  const WscRtpMode_Dvr({required this.currentTimeMs, required this.speed}): super._();
  

 final  PlatformInt64 currentTimeMs;
 final  double speed;

/// Create a copy of WscRtpMode
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpMode_DvrCopyWith<WscRtpMode_Dvr> get copyWith => _$WscRtpMode_DvrCopyWithImpl<WscRtpMode_Dvr>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpMode_Dvr&&(identical(other.currentTimeMs, currentTimeMs) || other.currentTimeMs == currentTimeMs)&&(identical(other.speed, speed) || other.speed == speed));
}


@override
int get hashCode => Object.hash(runtimeType,currentTimeMs,speed);

@override
String toString() {
  return 'WscRtpMode.dvr(currentTimeMs: $currentTimeMs, speed: $speed)';
}


}

/// @nodoc
abstract mixin class $WscRtpMode_DvrCopyWith<$Res> implements $WscRtpModeCopyWith<$Res> {
  factory $WscRtpMode_DvrCopyWith(WscRtpMode_Dvr value, $Res Function(WscRtpMode_Dvr) _then) = _$WscRtpMode_DvrCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 currentTimeMs, double speed
});




}
/// @nodoc
class _$WscRtpMode_DvrCopyWithImpl<$Res>
    implements $WscRtpMode_DvrCopyWith<$Res> {
  _$WscRtpMode_DvrCopyWithImpl(this._self, this._then);

  final WscRtpMode_Dvr _self;
  final $Res Function(WscRtpMode_Dvr) _then;

/// Create a copy of WscRtpMode
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? currentTimeMs = null,Object? speed = null,}) {
  return _then(WscRtpMode_Dvr(
currentTimeMs: null == currentTimeMs ? _self.currentTimeMs : currentTimeMs // ignore: cast_nullable_to_non_nullable
as PlatformInt64,speed: null == speed ? _self.speed : speed // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc
mixin _$WscRtpPlaybackEvent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WscRtpPlaybackEvent()';
}


}

/// @nodoc
class $WscRtpPlaybackEventCopyWith<$Res>  {
$WscRtpPlaybackEventCopyWith(WscRtpPlaybackEvent _, $Res Function(WscRtpPlaybackEvent) __);
}


/// Adds pattern-matching-related methods to [WscRtpPlaybackEvent].
extension WscRtpPlaybackEventPatterns on WscRtpPlaybackEvent {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( WscRtpPlaybackEvent_Playing value)?  playing,TResult Function( WscRtpPlaybackEvent_Waiting value)?  waiting,TResult Function( WscRtpPlaybackEvent_GapSkipped value)?  gapSkipped,TResult Function( WscRtpPlaybackEvent_Ended value)?  ended,TResult Function( WscRtpPlaybackEvent_Error value)?  error,TResult Function( WscRtpPlaybackEvent_RenegotiationRequired value)?  renegotiationRequired,required TResult orElse(),}){
final _that = this;
switch (_that) {
case WscRtpPlaybackEvent_Playing() when playing != null:
return playing(_that);case WscRtpPlaybackEvent_Waiting() when waiting != null:
return waiting(_that);case WscRtpPlaybackEvent_GapSkipped() when gapSkipped != null:
return gapSkipped(_that);case WscRtpPlaybackEvent_Ended() when ended != null:
return ended(_that);case WscRtpPlaybackEvent_Error() when error != null:
return error(_that);case WscRtpPlaybackEvent_RenegotiationRequired() when renegotiationRequired != null:
return renegotiationRequired(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( WscRtpPlaybackEvent_Playing value)  playing,required TResult Function( WscRtpPlaybackEvent_Waiting value)  waiting,required TResult Function( WscRtpPlaybackEvent_GapSkipped value)  gapSkipped,required TResult Function( WscRtpPlaybackEvent_Ended value)  ended,required TResult Function( WscRtpPlaybackEvent_Error value)  error,required TResult Function( WscRtpPlaybackEvent_RenegotiationRequired value)  renegotiationRequired,}){
final _that = this;
switch (_that) {
case WscRtpPlaybackEvent_Playing():
return playing(_that);case WscRtpPlaybackEvent_Waiting():
return waiting(_that);case WscRtpPlaybackEvent_GapSkipped():
return gapSkipped(_that);case WscRtpPlaybackEvent_Ended():
return ended(_that);case WscRtpPlaybackEvent_Error():
return error(_that);case WscRtpPlaybackEvent_RenegotiationRequired():
return renegotiationRequired(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( WscRtpPlaybackEvent_Playing value)?  playing,TResult? Function( WscRtpPlaybackEvent_Waiting value)?  waiting,TResult? Function( WscRtpPlaybackEvent_GapSkipped value)?  gapSkipped,TResult? Function( WscRtpPlaybackEvent_Ended value)?  ended,TResult? Function( WscRtpPlaybackEvent_Error value)?  error,TResult? Function( WscRtpPlaybackEvent_RenegotiationRequired value)?  renegotiationRequired,}){
final _that = this;
switch (_that) {
case WscRtpPlaybackEvent_Playing() when playing != null:
return playing(_that);case WscRtpPlaybackEvent_Waiting() when waiting != null:
return waiting(_that);case WscRtpPlaybackEvent_GapSkipped() when gapSkipped != null:
return gapSkipped(_that);case WscRtpPlaybackEvent_Ended() when ended != null:
return ended(_that);case WscRtpPlaybackEvent_Error() when error != null:
return error(_that);case WscRtpPlaybackEvent_RenegotiationRequired() when renegotiationRequired != null:
return renegotiationRequired(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PlatformInt64 timestamp)?  playing,TResult Function( PlatformInt64 timestamp)?  waiting,TResult Function( PlatformInt64 from,  PlatformInt64 to)?  gapSkipped,TResult Function( PlatformInt64 timestamp)?  ended,TResult Function( PlatformInt64 timestamp,  String message)?  error,TResult Function( String codec,  String message)?  renegotiationRequired,required TResult orElse(),}) {final _that = this;
switch (_that) {
case WscRtpPlaybackEvent_Playing() when playing != null:
return playing(_that.timestamp);case WscRtpPlaybackEvent_Waiting() when waiting != null:
return waiting(_that.timestamp);case WscRtpPlaybackEvent_GapSkipped() when gapSkipped != null:
return gapSkipped(_that.from,_that.to);case WscRtpPlaybackEvent_Ended() when ended != null:
return ended(_that.timestamp);case WscRtpPlaybackEvent_Error() when error != null:
return error(_that.timestamp,_that.message);case WscRtpPlaybackEvent_RenegotiationRequired() when renegotiationRequired != null:
return renegotiationRequired(_that.codec,_that.message);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PlatformInt64 timestamp)  playing,required TResult Function( PlatformInt64 timestamp)  waiting,required TResult Function( PlatformInt64 from,  PlatformInt64 to)  gapSkipped,required TResult Function( PlatformInt64 timestamp)  ended,required TResult Function( PlatformInt64 timestamp,  String message)  error,required TResult Function( String codec,  String message)  renegotiationRequired,}) {final _that = this;
switch (_that) {
case WscRtpPlaybackEvent_Playing():
return playing(_that.timestamp);case WscRtpPlaybackEvent_Waiting():
return waiting(_that.timestamp);case WscRtpPlaybackEvent_GapSkipped():
return gapSkipped(_that.from,_that.to);case WscRtpPlaybackEvent_Ended():
return ended(_that.timestamp);case WscRtpPlaybackEvent_Error():
return error(_that.timestamp,_that.message);case WscRtpPlaybackEvent_RenegotiationRequired():
return renegotiationRequired(_that.codec,_that.message);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PlatformInt64 timestamp)?  playing,TResult? Function( PlatformInt64 timestamp)?  waiting,TResult? Function( PlatformInt64 from,  PlatformInt64 to)?  gapSkipped,TResult? Function( PlatformInt64 timestamp)?  ended,TResult? Function( PlatformInt64 timestamp,  String message)?  error,TResult? Function( String codec,  String message)?  renegotiationRequired,}) {final _that = this;
switch (_that) {
case WscRtpPlaybackEvent_Playing() when playing != null:
return playing(_that.timestamp);case WscRtpPlaybackEvent_Waiting() when waiting != null:
return waiting(_that.timestamp);case WscRtpPlaybackEvent_GapSkipped() when gapSkipped != null:
return gapSkipped(_that.from,_that.to);case WscRtpPlaybackEvent_Ended() when ended != null:
return ended(_that.timestamp);case WscRtpPlaybackEvent_Error() when error != null:
return error(_that.timestamp,_that.message);case WscRtpPlaybackEvent_RenegotiationRequired() when renegotiationRequired != null:
return renegotiationRequired(_that.codec,_that.message);case _:
  return null;

}
}

}

/// @nodoc


class WscRtpPlaybackEvent_Playing extends WscRtpPlaybackEvent {
  const WscRtpPlaybackEvent_Playing({required this.timestamp}): super._();
  

 final  PlatformInt64 timestamp;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpPlaybackEvent_PlayingCopyWith<WscRtpPlaybackEvent_Playing> get copyWith => _$WscRtpPlaybackEvent_PlayingCopyWithImpl<WscRtpPlaybackEvent_Playing>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent_Playing&&(identical(other.timestamp, timestamp) || other.timestamp == timestamp));
}


@override
int get hashCode => Object.hash(runtimeType,timestamp);

@override
String toString() {
  return 'WscRtpPlaybackEvent.playing(timestamp: $timestamp)';
}


}

/// @nodoc
abstract mixin class $WscRtpPlaybackEvent_PlayingCopyWith<$Res> implements $WscRtpPlaybackEventCopyWith<$Res> {
  factory $WscRtpPlaybackEvent_PlayingCopyWith(WscRtpPlaybackEvent_Playing value, $Res Function(WscRtpPlaybackEvent_Playing) _then) = _$WscRtpPlaybackEvent_PlayingCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 timestamp
});




}
/// @nodoc
class _$WscRtpPlaybackEvent_PlayingCopyWithImpl<$Res>
    implements $WscRtpPlaybackEvent_PlayingCopyWith<$Res> {
  _$WscRtpPlaybackEvent_PlayingCopyWithImpl(this._self, this._then);

  final WscRtpPlaybackEvent_Playing _self;
  final $Res Function(WscRtpPlaybackEvent_Playing) _then;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? timestamp = null,}) {
  return _then(WscRtpPlaybackEvent_Playing(
timestamp: null == timestamp ? _self.timestamp : timestamp // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class WscRtpPlaybackEvent_Waiting extends WscRtpPlaybackEvent {
  const WscRtpPlaybackEvent_Waiting({required this.timestamp}): super._();
  

 final  PlatformInt64 timestamp;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpPlaybackEvent_WaitingCopyWith<WscRtpPlaybackEvent_Waiting> get copyWith => _$WscRtpPlaybackEvent_WaitingCopyWithImpl<WscRtpPlaybackEvent_Waiting>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent_Waiting&&(identical(other.timestamp, timestamp) || other.timestamp == timestamp));
}


@override
int get hashCode => Object.hash(runtimeType,timestamp);

@override
String toString() {
  return 'WscRtpPlaybackEvent.waiting(timestamp: $timestamp)';
}


}

/// @nodoc
abstract mixin class $WscRtpPlaybackEvent_WaitingCopyWith<$Res> implements $WscRtpPlaybackEventCopyWith<$Res> {
  factory $WscRtpPlaybackEvent_WaitingCopyWith(WscRtpPlaybackEvent_Waiting value, $Res Function(WscRtpPlaybackEvent_Waiting) _then) = _$WscRtpPlaybackEvent_WaitingCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 timestamp
});




}
/// @nodoc
class _$WscRtpPlaybackEvent_WaitingCopyWithImpl<$Res>
    implements $WscRtpPlaybackEvent_WaitingCopyWith<$Res> {
  _$WscRtpPlaybackEvent_WaitingCopyWithImpl(this._self, this._then);

  final WscRtpPlaybackEvent_Waiting _self;
  final $Res Function(WscRtpPlaybackEvent_Waiting) _then;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? timestamp = null,}) {
  return _then(WscRtpPlaybackEvent_Waiting(
timestamp: null == timestamp ? _self.timestamp : timestamp // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class WscRtpPlaybackEvent_GapSkipped extends WscRtpPlaybackEvent {
  const WscRtpPlaybackEvent_GapSkipped({required this.from, required this.to}): super._();
  

 final  PlatformInt64 from;
 final  PlatformInt64 to;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpPlaybackEvent_GapSkippedCopyWith<WscRtpPlaybackEvent_GapSkipped> get copyWith => _$WscRtpPlaybackEvent_GapSkippedCopyWithImpl<WscRtpPlaybackEvent_GapSkipped>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent_GapSkipped&&(identical(other.from, from) || other.from == from)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode => Object.hash(runtimeType,from,to);

@override
String toString() {
  return 'WscRtpPlaybackEvent.gapSkipped(from: $from, to: $to)';
}


}

/// @nodoc
abstract mixin class $WscRtpPlaybackEvent_GapSkippedCopyWith<$Res> implements $WscRtpPlaybackEventCopyWith<$Res> {
  factory $WscRtpPlaybackEvent_GapSkippedCopyWith(WscRtpPlaybackEvent_GapSkipped value, $Res Function(WscRtpPlaybackEvent_GapSkipped) _then) = _$WscRtpPlaybackEvent_GapSkippedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 from, PlatformInt64 to
});




}
/// @nodoc
class _$WscRtpPlaybackEvent_GapSkippedCopyWithImpl<$Res>
    implements $WscRtpPlaybackEvent_GapSkippedCopyWith<$Res> {
  _$WscRtpPlaybackEvent_GapSkippedCopyWithImpl(this._self, this._then);

  final WscRtpPlaybackEvent_GapSkipped _self;
  final $Res Function(WscRtpPlaybackEvent_GapSkipped) _then;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? to = null,}) {
  return _then(WscRtpPlaybackEvent_GapSkipped(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as PlatformInt64,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class WscRtpPlaybackEvent_Ended extends WscRtpPlaybackEvent {
  const WscRtpPlaybackEvent_Ended({required this.timestamp}): super._();
  

 final  PlatformInt64 timestamp;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpPlaybackEvent_EndedCopyWith<WscRtpPlaybackEvent_Ended> get copyWith => _$WscRtpPlaybackEvent_EndedCopyWithImpl<WscRtpPlaybackEvent_Ended>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent_Ended&&(identical(other.timestamp, timestamp) || other.timestamp == timestamp));
}


@override
int get hashCode => Object.hash(runtimeType,timestamp);

@override
String toString() {
  return 'WscRtpPlaybackEvent.ended(timestamp: $timestamp)';
}


}

/// @nodoc
abstract mixin class $WscRtpPlaybackEvent_EndedCopyWith<$Res> implements $WscRtpPlaybackEventCopyWith<$Res> {
  factory $WscRtpPlaybackEvent_EndedCopyWith(WscRtpPlaybackEvent_Ended value, $Res Function(WscRtpPlaybackEvent_Ended) _then) = _$WscRtpPlaybackEvent_EndedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 timestamp
});




}
/// @nodoc
class _$WscRtpPlaybackEvent_EndedCopyWithImpl<$Res>
    implements $WscRtpPlaybackEvent_EndedCopyWith<$Res> {
  _$WscRtpPlaybackEvent_EndedCopyWithImpl(this._self, this._then);

  final WscRtpPlaybackEvent_Ended _self;
  final $Res Function(WscRtpPlaybackEvent_Ended) _then;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? timestamp = null,}) {
  return _then(WscRtpPlaybackEvent_Ended(
timestamp: null == timestamp ? _self.timestamp : timestamp // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class WscRtpPlaybackEvent_Error extends WscRtpPlaybackEvent {
  const WscRtpPlaybackEvent_Error({required this.timestamp, required this.message}): super._();
  

 final  PlatformInt64 timestamp;
 final  String message;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpPlaybackEvent_ErrorCopyWith<WscRtpPlaybackEvent_Error> get copyWith => _$WscRtpPlaybackEvent_ErrorCopyWithImpl<WscRtpPlaybackEvent_Error>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent_Error&&(identical(other.timestamp, timestamp) || other.timestamp == timestamp)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,timestamp,message);

@override
String toString() {
  return 'WscRtpPlaybackEvent.error(timestamp: $timestamp, message: $message)';
}


}

/// @nodoc
abstract mixin class $WscRtpPlaybackEvent_ErrorCopyWith<$Res> implements $WscRtpPlaybackEventCopyWith<$Res> {
  factory $WscRtpPlaybackEvent_ErrorCopyWith(WscRtpPlaybackEvent_Error value, $Res Function(WscRtpPlaybackEvent_Error) _then) = _$WscRtpPlaybackEvent_ErrorCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 timestamp, String message
});




}
/// @nodoc
class _$WscRtpPlaybackEvent_ErrorCopyWithImpl<$Res>
    implements $WscRtpPlaybackEvent_ErrorCopyWith<$Res> {
  _$WscRtpPlaybackEvent_ErrorCopyWithImpl(this._self, this._then);

  final WscRtpPlaybackEvent_Error _self;
  final $Res Function(WscRtpPlaybackEvent_Error) _then;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? timestamp = null,Object? message = null,}) {
  return _then(WscRtpPlaybackEvent_Error(
timestamp: null == timestamp ? _self.timestamp : timestamp // ignore: cast_nullable_to_non_nullable
as PlatformInt64,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WscRtpPlaybackEvent_RenegotiationRequired extends WscRtpPlaybackEvent {
  const WscRtpPlaybackEvent_RenegotiationRequired({required this.codec, required this.message}): super._();
  

 final  String codec;
 final  String message;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WscRtpPlaybackEvent_RenegotiationRequiredCopyWith<WscRtpPlaybackEvent_RenegotiationRequired> get copyWith => _$WscRtpPlaybackEvent_RenegotiationRequiredCopyWithImpl<WscRtpPlaybackEvent_RenegotiationRequired>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WscRtpPlaybackEvent_RenegotiationRequired&&(identical(other.codec, codec) || other.codec == codec)&&(identical(other.message, message) || other.message == message));
}


@override
int get hashCode => Object.hash(runtimeType,codec,message);

@override
String toString() {
  return 'WscRtpPlaybackEvent.renegotiationRequired(codec: $codec, message: $message)';
}


}

/// @nodoc
abstract mixin class $WscRtpPlaybackEvent_RenegotiationRequiredCopyWith<$Res> implements $WscRtpPlaybackEventCopyWith<$Res> {
  factory $WscRtpPlaybackEvent_RenegotiationRequiredCopyWith(WscRtpPlaybackEvent_RenegotiationRequired value, $Res Function(WscRtpPlaybackEvent_RenegotiationRequired) _then) = _$WscRtpPlaybackEvent_RenegotiationRequiredCopyWithImpl;
@useResult
$Res call({
 String codec, String message
});




}
/// @nodoc
class _$WscRtpPlaybackEvent_RenegotiationRequiredCopyWithImpl<$Res>
    implements $WscRtpPlaybackEvent_RenegotiationRequiredCopyWith<$Res> {
  _$WscRtpPlaybackEvent_RenegotiationRequiredCopyWithImpl(this._self, this._then);

  final WscRtpPlaybackEvent_RenegotiationRequired _self;
  final $Res Function(WscRtpPlaybackEvent_RenegotiationRequired) _then;

/// Create a copy of WscRtpPlaybackEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? codec = null,Object? message = null,}) {
  return _then(WscRtpPlaybackEvent_RenegotiationRequired(
codec: null == codec ? _self.codec : codec // ignore: cast_nullable_to_non_nullable
as String,message: null == message ? _self.message : message // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
