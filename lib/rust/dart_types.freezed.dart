// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
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




// dart format on
