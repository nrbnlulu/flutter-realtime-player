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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( StreamEvent_Error value)?  error,TResult Function( StreamEvent_CurrentTime value)?  currentTime,TResult Function( StreamEvent_OriginVideoSize value)?  originVideoSize,TResult Function( StreamEvent_WscRtpSessionMode value)?  wscRtpSessionMode,TResult Function( StreamEvent_WscRtpStreamState value)?  wscRtpStreamState,required TResult orElse(),}){
final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that);case StreamEvent_WscRtpStreamState() when wscRtpStreamState != null:
return wscRtpStreamState(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( StreamEvent_Error value)  error,required TResult Function( StreamEvent_CurrentTime value)  currentTime,required TResult Function( StreamEvent_OriginVideoSize value)  originVideoSize,required TResult Function( StreamEvent_WscRtpSessionMode value)  wscRtpSessionMode,required TResult Function( StreamEvent_WscRtpStreamState value)  wscRtpStreamState,}){
final _that = this;
switch (_that) {
case StreamEvent_Error():
return error(_that);case StreamEvent_CurrentTime():
return currentTime(_that);case StreamEvent_OriginVideoSize():
return originVideoSize(_that);case StreamEvent_WscRtpSessionMode():
return wscRtpSessionMode(_that);case StreamEvent_WscRtpStreamState():
return wscRtpStreamState(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( StreamEvent_Error value)?  error,TResult? Function( StreamEvent_CurrentTime value)?  currentTime,TResult? Function( StreamEvent_OriginVideoSize value)?  originVideoSize,TResult? Function( StreamEvent_WscRtpSessionMode value)?  wscRtpSessionMode,TResult? Function( StreamEvent_WscRtpStreamState value)?  wscRtpStreamState,}){
final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that);case StreamEvent_WscRtpStreamState() when wscRtpStreamState != null:
return wscRtpStreamState(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String field0)?  error,TResult Function( PlatformInt64 field0)?  currentTime,TResult Function( BigInt width,  BigInt height)?  originVideoSize,TResult Function( WscRtpMode field0)?  wscRtpSessionMode,TResult Function( String field0)?  wscRtpStreamState,required TResult orElse(),}) {final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that.field0);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that.field0);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that.width,_that.height);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that.field0);case StreamEvent_WscRtpStreamState() when wscRtpStreamState != null:
return wscRtpStreamState(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String field0)  error,required TResult Function( PlatformInt64 field0)  currentTime,required TResult Function( BigInt width,  BigInt height)  originVideoSize,required TResult Function( WscRtpMode field0)  wscRtpSessionMode,required TResult Function( String field0)  wscRtpStreamState,}) {final _that = this;
switch (_that) {
case StreamEvent_Error():
return error(_that.field0);case StreamEvent_CurrentTime():
return currentTime(_that.field0);case StreamEvent_OriginVideoSize():
return originVideoSize(_that.width,_that.height);case StreamEvent_WscRtpSessionMode():
return wscRtpSessionMode(_that.field0);case StreamEvent_WscRtpStreamState():
return wscRtpStreamState(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String field0)?  error,TResult? Function( PlatformInt64 field0)?  currentTime,TResult? Function( BigInt width,  BigInt height)?  originVideoSize,TResult? Function( WscRtpMode field0)?  wscRtpSessionMode,TResult? Function( String field0)?  wscRtpStreamState,}) {final _that = this;
switch (_that) {
case StreamEvent_Error() when error != null:
return error(_that.field0);case StreamEvent_CurrentTime() when currentTime != null:
return currentTime(_that.field0);case StreamEvent_OriginVideoSize() when originVideoSize != null:
return originVideoSize(_that.width,_that.height);case StreamEvent_WscRtpSessionMode() when wscRtpSessionMode != null:
return wscRtpSessionMode(_that.field0);case StreamEvent_WscRtpStreamState() when wscRtpStreamState != null:
return wscRtpStreamState(_that.field0);case _:
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


class StreamEvent_WscRtpStreamState extends StreamEvent {
  const StreamEvent_WscRtpStreamState(this.field0): super._();
  

 final  String field0;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamEvent_WscRtpStreamStateCopyWith<StreamEvent_WscRtpStreamState> get copyWith => _$StreamEvent_WscRtpStreamStateCopyWithImpl<StreamEvent_WscRtpStreamState>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamEvent_WscRtpStreamState&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'StreamEvent.wscRtpStreamState(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $StreamEvent_WscRtpStreamStateCopyWith<$Res> implements $StreamEventCopyWith<$Res> {
  factory $StreamEvent_WscRtpStreamStateCopyWith(StreamEvent_WscRtpStreamState value, $Res Function(StreamEvent_WscRtpStreamState) _then) = _$StreamEvent_WscRtpStreamStateCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$StreamEvent_WscRtpStreamStateCopyWithImpl<$Res>
    implements $StreamEvent_WscRtpStreamStateCopyWith<$Res> {
  _$StreamEvent_WscRtpStreamStateCopyWithImpl(this._self, this._then);

  final StreamEvent_WscRtpStreamState _self;
  final $Res Function(StreamEvent_WscRtpStreamState) _then;

/// Create a copy of StreamEvent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(StreamEvent_WscRtpStreamState(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
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

// dart format on
