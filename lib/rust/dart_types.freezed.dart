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
  const StreamState_Playing({required this.textureId}): super._();
  

 final  PlatformInt64 textureId;

/// Create a copy of StreamState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$StreamState_PlayingCopyWith<StreamState_Playing> get copyWith => _$StreamState_PlayingCopyWithImpl<StreamState_Playing>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is StreamState_Playing&&(identical(other.textureId, textureId) || other.textureId == textureId));
}


@override
int get hashCode => Object.hash(runtimeType,textureId);

@override
String toString() {
  return 'StreamState.playing(textureId: $textureId)';
}


}

/// @nodoc
abstract mixin class $StreamState_PlayingCopyWith<$Res> implements $StreamStateCopyWith<$Res> {
  factory $StreamState_PlayingCopyWith(StreamState_Playing value, $Res Function(StreamState_Playing) _then) = _$StreamState_PlayingCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 textureId
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
@pragma('vm:prefer-inline') $Res call({Object? textureId = null,}) {
  return _then(StreamState_Playing(
textureId: null == textureId ? _self.textureId : textureId // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
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
